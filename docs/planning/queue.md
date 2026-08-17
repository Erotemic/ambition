# The queue — standing execution ledger

**This file is the SPINE and the ledger `scripts/goal_guard.py` reads.** It is
intentionally self-replenishing. The literal open marker `▢` appears in this
header as well as on executable rows, so the guard never interprets an empty
snapshot of the queue as permission to stop.

Its name carries no date on purpose. This ledger outlives any one run: it was
`queue-72h-2026-08-08.md` until 2026-08-15, and a dated name on a file whose
whole property is that it never closes is how the guard's pointers went stale
before. A run rotates; the ledger does not. ⛔ If this file is ever renamed or
archived anyway, repoint `.goal/active.json` in the SAME commit — its checks
name this path, and a check whose subject vanished is the one failure that
looks like success.

> **Finish work, then promote the next highest-value verified item and keep
> going. There is no "the queue is empty, therefore stop" state.**

Document roles:

- **this queue** owns current execution order;
- [`tracks.md`](tracks.md) is the standing reservoir;
- focused plans own technical design and acceptance criteria;
- [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
  owns direct maintainer observations;
- [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) owns
  questions that genuinely require a product/feel decision;
- completed campaigns and migration evidence belong in `docs/archive`.

Before starting a row, inspect HEAD and confirm the named gap still exists. When
a row closes, remove its historical case file from this ledger, preserve useful
history in the archive if needed, promote another verified item, and continue.

D73 is closed and its working-memory documents are archived under
[`../archive/planning-superseded/2026-08-13/`](../archive/planning-superseded/2026-08-13/).
The successor strategy is
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).
Do not reopen deleted character/archetype authority merely because archived
migration prose names it.

---

## Current execution order

### ✔ LANDED 2026-08-15 — six worker lanes, all merged, validated and pushed

⚠ **this block is history, not work.** Kept because each row's *evidence* is
where a later session should look before reopening any of it.

| Lane | What landed | Proof |
|---|---|---|
| D125 | cross-room occurrence continuity: a `Placed` row suppresses the home room and reinstates where the object lies, as ONE decision | 6/6 acceptance; **both** poisons red (revert the arm → duplication; delete the foreign leg → ZERO, the deletion bug) |
| Mary-O LDtk | `mary_o_1_3` authored end to end through LDtk; four hand-kept registration sites deleted | the honest headline was *"almost nothing needed inventing"* |
| Smash CPU | one instrument, histogram prints every run | George vs duelist MEASURED: 6 vs 8 distinct, specials 9/4 vs 0/0, aerials 3/10 vs 6/9 |
| VFX | `HostVfxPresentationPlugin` — four demo apps were writing `VfxMessage` into a queue with NO READER | withheld the plugin → the demo's own VFX test goes red |
| LDtk contract | `ldtk_entity_contract.json`: one table, Rust prover runs the real converters against it in BOTH directions | caught an undeclared `MovingPlatform.speed`, then caught the coordinator's wrong fix for it |
| `next_room` + tubes | the exit chain and warp tubes are authored content, not Rust control flow | poison → exactly one test red |

⭐ **the two engine-level lessons, because they generalise past their lanes:**
a **generator that owns a whole file discards anything authored by another
road** — a regenerate deleted an entire level while every check stayed green,
which is why `scripts/check_authored_levels_survive.py` now ratchets the level
roster; and **a construction test pins the FUNCTION, not the WIRING** — the
facing plumbing was green the whole time enemies walked the wrong way, because
nothing asserted the authored world ever *said* which way.

⚠ **peer agents commit to this same main tree.** ⛔⛔ therefore every commit uses
`git commit -F - -- <paths>`; a bare `git commit` takes the WHOLE INDEX and
carries another session's staged files under this one's message.

⚠ **treat every worker test claim as UNRUN until this session runs it** — two of
those six lanes handed back code that did not compile, and one handed back a
confident diagnosis a five-minute source read overturned. That cost is
independent of whether the worker could build.

### ▢ CURRENT LANES — two, as of 2026-08-15 (the six above are HISTORY)

⛔ **the "three lanes dispatched" table this file used to carry is gone**: all
three had landed and no worker was running, so the ledger was describing a
workforce that did not exist. Refill this table when a lane returns; never leave
it describing the last run.

| Lane | Owner | Executable next action |
|---|---|---|
| **D125 — checkpoint/reset truth over persistent occurrences** | coordinator, in the main tree | the seven-step behavioural fixture below |
| **D128 — Smash CPU showcase** | one worker, in its own worktree | measure real matches first, then spend on the single largest visible deficiency |

⭐ **the build lease is no longer exclusive, and the reason changed.** The old
rule — *workers never run `cargo`* — rested on one shared target dir against a
nearly full disk. Both halves are now false: `scripts/setup_target_bindmount.sh`
gives each worktree its own ext4 backing store keyed by path, and the stale dirs
are deleted. ⇒ a worker whose job **is** measurement (the Smash lane cannot
observe a match without running it) gets a worktree and builds in it. The
surviving cost is CPU contention, which is a scheduling choice, not a limit.

### ▢ Next dispatch — maintainer-reported product bugs still unmarked

⭐ these are Jon's own sentences in
[`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
with **no marker at all**, which means nobody has even ruled on them. Promote one
whenever a lane returns; ⛔ do not let a lane finish with nothing dispatched.

| Observation | Why it is worth a lane |
|---|---|
| Super Sanic's spikes are clipped by the sprite renderer | ⭐ Jon called it structural himself — *"we should not be able to clip sprite artwork so easily"*. This is the only one that is an ENGINE gap rather than content |
| Mary-O secret/invisible blocks keep their brick texture when spent (quasar brick in 1-1) | the spent-art road already works for `?`-blocks, so this is a road that skipped a case |
| Mary-O allows one fireball; should allow two | small, and the number is content, not engine |
| ~~the multi-coin block's coin-pop VFX~~ | ✔ **RESOLVED 2026-08-15 and it was never missing** — it landed in `943a9aa0c`; four demo shells had no `VfxMessage` reader, so it drew in the full game and nowhere else. ⛔ the doc entry said otherwise for a day |
| the snake and AI slop are far too big, and the snake sprite may not match its box | ⚠ related to the player-side sprite/box unit mismatch at the top of that file — the two may be one bug |
| **Sanic is very small in his own game** (Jon, 2026-08-15) | ⭐⭐ **third body in the sprite/box cluster, and the one that makes it a CLUSTER rather than three bugs** — see the measurement below |
| **drop `pocket` and `versus` from the main game-selection shell** (Jon, 2026-08-15) — *"They can just be standalone exes for tests."* | ⭐ a SHELL-COMPOSITION change, not a deletion: both keep their binaries and their suites. ⚠ the shell's roster is the thing a player sees first, and two test fixtures sitting in it is the demo gate leaking the other way — the shell advertising what the engine can compose rather than what the player can play |

### ▢ Two things found in passing 2026-08-15, logged rather than fixed

**1. ⚠ TWO WORLDS FAIL LDtk VALIDATION TODAY, and the tool writes them anyway.**
`sandbox.ldtk` and `mary_o.ldtk` emit `error:` diagnostics on every edit —
cross-world `LoadingZone` targets (intro names sandbox's rooms and vice versa, so
a SINGLE-FILE validator cannot resolve either) and `MaryOBlock`, which no entity
manifest declares. ⛔⛔ **the errors do not block the write**, which is how they
cost a correction: three `error:` lines filled a `| head -3` and hid the
`wrote` line under them, so an edit that HAD landed was reported as refused.
⇒ either the cross-world case needs a world-SET validator, or those targets need
declaring; and `MaryOBlock` needs a manifest row. ⚠ neither is urgent, and both
make every future LDtk edit noisier and less trustworthy than it should be.

**2. ⛔ THE SMASH LANE'S VISUAL FIXES SHIP UNSEEN — Jon's eyes are the only
instrument.** The camera-close ease, the 3-2-1-GO card and the winner card are
all measured (`close 360.9 → 68.9`) and none has been LOOKED at. Specifically
unverified: that a centred 34pt card actually renders at
`SMASH_ANNOUNCE_HUD_SLOT` in the hosted app, and that 5 Hz is the right close
rate — that number is a judgement, not a measurement. ⇒ **worth one CPU-vs-CPU
match watched by a human**, and cheap to correct if wrong.

⭐⭐ **THE SPRITE/BOX CLUSTER HAS A MEASURED SHAPE — started 2026-08-15, NOT
finished.** Three reports (snake too big · player hurtbox mismatched · Sanic too
small) are one question: **there are TWO sizing roads and a body's size depends
on which one it is on.**

```text
published road   collision derived from the sprite's own body_metrics,
                 quad size stated explicitly by ActorRenderSize
legacy road      collision * collision_scale, a hand-tuned per-character
                 number in character_catalog.ron ranging 1.15 .. 2.1
```

`ActorRenderSize`'s own doc says it: *"Absent ⇒ the actor uses the legacy
`collision_scale` render path."*

⭐ **measured: 194 of 196 spritesheet specs already publish `body_metrics`**, so
the DATA is not the gap.

⚠⚠ **THE NEXT ISSUE THIS EXPOSED — A BODY WHOSE COLLISION SIZE COULD DEPEND ON
THE GRAPHICS SETTING.** **Twelve sheets declare `authored_body: true` in every
reduced quality tier and NOT at full resolution** — `player_extended`, the three
`player_*_review` sheets, and eight `robot_*` variants (37 authored in
`sprites/`, 49 in each tier). ⛔ not a rounding artifact: `player_extended` reads
**67 × 165** at full res against **35 × 84** in `0_5x`, i.e. 70 × 168 doubled — a
genuinely different rectangle. Mtimes say the TIERS are newer (0_5x 2026-08-08 vs
full-res 2026-07-29), so `regen_visual_quality_variants.sh` applies a generator
`body_inset` the full-res road was never regenerated to carry. ⭐ today it is
latent only by luck — `authored_body_pixel_size` is called with the bare target
id, so full-res always wins — and the rot widens every time one road is
regenerated without the other. ⇒ owed: regenerate those twelve at full res, and
tie the tiers' `body_pixel_bbox` to the full-res one by scale so they cannot
diverge again.

✔✔ **THE ADOPTION COUNT IS IN (2026-08-16), AND IT IS THE WHOLE ANSWER: TWO.**
The decision site is `posed_body_for` in
`character_runtime/presentation.rs` — a definition authoring
`BodySource::SpriteAuthored { world_per_pixel }` gets `SpritePosedBody`, and
`sync_sprite_posed_bodies` then keeps its collision box, sprite quad and quad
offset all derived from the sheet, *"so none of the three can drift from the
other two"*. `BodySource::Explicit` and `None` get nothing and fall to the legacy
path.

```text
with_sprite_authored_body callers   2   (player_robot_lineage, mary_o)
character_catalog.ron rows with a hand-tuned collision_scale   33   (1.1 .. 4.5)
sheets publishing the body_metrics the good road needs        194 of 196
```

⇒ ⭐⭐ **`world_per_pixel` IS the common unit Jon's hurtbox note says was never
established** — one number saying how much world a sheet pixel covers.

⛔⛔ **CORRECTION 2026-08-16 — A PARAGRAPH HERE CLAIMED "only 6 of 194 sheets
declare `authored_body: true`, and `player_robot_v3` is not one of them". BOTH
HALVES WERE FALSE, and it was a TRUNCATED SEARCH, not a wrong reading.** The
scan matched `authored_body` only within the first 400 characters after
`body_metrics: Some(`; in v3's record the flag sits **2,070 characters in**,
after the per-animation table. Measured with no window:

```text
sheets with body_metrics            194
declaring authored_body: true        37     (NOT 6)
player_robot_v3 among them          YES     — and `robot`, `player_robot_v2`,
                                              noether, alice, bob, and 30 more
```

⇒ the player's call site is **not dormant, it fires today**, and the lineage
additionally hangs a `forgiving_hurtbox` off it. ⛔ **the third truncation error
of this campaign, and the second to reach Jon as a finding.** Do not report an
absence from a windowed search.

✔ **and the report it was aimed at is ALREADY FIXED** (renderer `dd744b4`,
*"v3 authors his collision box instead of measuring the idle alpha bbox"*).
Measured on the shipped sheet: authored body **57 × 91** against a drawn idle
silhouette of **71 × 103** — 7 px clear of each arm, 10 px under the antenna,
2 px above the shoe line, **0.71× the area**. Jon reported the old box at
1.28× wide / 1.29× tall; it is now 0.80× / 0.88×.

⛔⛔ **BUT NOTHING COULD SEE THAT, WHICH IS THE REAL GAP AND IT IS NOW CLOSED**
(`4213db3d4`). Two tests already pinned that box — one against standing height,
one against the hurtbox it carries — and **poisoning `body_pixel_bbox` back out
to the full silhouette leaves BOTH GREEN**: the height still resolves, and a
hurtbox that is a fraction of whatever box it is handed is still a fraction. The
one claim nobody checked was the claim in the report. ⭐ the new test compares
the two rectangles **data-to-data**: the atlas packer already trims every frame
to its opaque bbox and records where it sat, so the union of a row's `off`/`w`/`h`
IS the silhouette, in `body_pixel_bbox`'s own pixel space — no PNG decode, no
magic numbers, survives a redraw. Falsified both ways, and the vacuity guard is
the one that matters: an untrimmed frame reports the whole 256×256 logical frame
and would wave through a body box of any size.

⚠ **what SURVIVES the correction**: `with_sprite_authored_body` still has **two
character adopters** (v3 and Mary-O) against 33 hand-tuned `collision_scale`
rows, so the cluster's shape is unchanged — the count was wrong, the diagnosis
was not. And **the snake and Sanic genuinely do not declare an authored body**
(`solid_snake`, both `snakes_on_a_*`, `sanic`, `super_sanic` all measured; only
Sanic's two PROPS are authored), so those two reports really are the same bug and
the fix is the same three-line edit in each renderer target.

⛔ **do not fix Sanic's scale in isolation** — a fourth hand-tuned constant is
what this cluster is made of. ⚠ and ⛔ do not delete `collision_scale` before
counting: a shipped capability can have zero adopters, and so can a legacy path
still carrying half the roster.

⚠ **the sprite/box pair is the cluster worth taking together**: the player hurtbox
and the snake box both come down to sprite and collision numbers never having
been converted to a common unit.

- ⏸ **D117 — Finish the controlled-character actor kernel. RESTING, blocked on
  the hit-emphasis decision.**

⛔ **this row said "the current architecture priority" while the focused plan
said the system is at a reasonable resting point.** Both cannot be the execution
authority, and agents were silently skipping the row rather than resolving it.
Resolved 2026-08-14 in favour of the plan, because the plan is the one measured
against HEAD: control authority CONVERGED (one `tick_controlled_brains`), and
`tick_actor_brains` reads as a sequence after three extractions. The only
structural work left in this milestone is the movement/TIME integrator fork,
which needs the hit-emphasis/proper-time decision — [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) **#6** (renumbered 2026-08-14; that file once had two sections numbered 5).

⛔ **do not manufacture another helper extraction to make the function shorter.**
"Bevy accepts the signature" was never the goal, and neither is a line count. If
a concrete semantic phase extraction appears that reduces mixed authority AND
unblocks another subsystem AND needs no time-integration decision, take it and
say so here; otherwise this row waits. The top executable rows are D116 and D118.

Use
[`engine/controlled-character-actor-kernel.md`](engine/controlled-character-actor-kernel.md),
[`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md)
and [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).

Start with the generic actor-brain/crowd/control path. Remove the hidden
`PrimaryPlayer` coordinate system from generic arbitration, split world
observation/decision/mutation by semantic phase rather than tuple-packing Bevy
parameters, and make controlled/AI bodies use one ordinary body/control contract.
Do not start a broad file-move campaign before this ownership boundary is true.

Progress against that milestone is tracked in the focused plan, not here. ⛔
**read it before starting a slice** — a row that looks more complete than the
code is the most expensive kind of stale, and so is one that looks less.

⚠ **the hit-emphasis decision blocks TIME INTEGRATION only.** Control authority
converged on 2026-08-14 without it (one `tick_controlled_brains`); merging
`integrate_home_body` with `integrate_actor_body` still waits on it. Do not let
the feel decision be quoted as a blocker on unrelated control-authority work.

⭐ the milestone already delivered what the other programs needed: D115's
moving-world work, D116 multiplayer and the persistent-world programs all build
on an ordinary controlled-body kernel now rather than around a protagonist-
special simulation.

- ⏸ **D118 — Per-view camera reference frames. REST ROW — its remainder lives in
  D116.**

The camera-frame implementation is COMPLETE: subject-relative roll, rotated
viewport clamping, safe-area framing in screen axes, roll easing with portal-seam
adoption, and view-owned policy (`CameraReferenceFrame` is a component on the
local view). ⛔ **do not continue it as a standalone campaign.** C5 — camera policy
read off the view index — is N-VIEW work and belongs to D116; the feel questions
(shake units, acceptance customers) are filed in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). ⭐ **a row
whose remainder lives in another row is a rest row, not a campaign.**

Design lives in
[`engine/camera-reference-frame-policy.md`](engine/camera-reference-frame-policy.md);
the discharged case file is archived at
[`../archive/planning-superseded/2026-08-14/d118-camera-reference-frames.md`](../archive/planning-superseded/2026-08-14/d118-camera-reference-frames.md).

- ⏸ **D115 — Ambition-first LDtk authoring + moving-platform architecture. RESTING: K2–K6 all closed.**

Design: [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
and [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).
Execution detail archived as evidence:
[`../archive/planning-superseded/2026-08-15/d115-ldtk-authoring-and-kinematic-world.md`](../archive/planning-superseded/2026-08-15/d115-ldtk-authoring-and-kinematic-world.md).

✔ **K2–K4** typed path references, the ownership carve, contact completeness.
✔ **K5** native `path_ref` `EntityRef`; `Patrol:` gone from every shipped world;
**−347-line validator**. ✔ **K6 closed ON EVIDENCE, not by adoption**: the second
dynamic-geometry customer is the **door**, it has shipped for months, and it is
**not kinematic** — it appears, it does not slide. `MovingPlatformState` is still
the only writer of a non-zero `Block::velocity`; the shortage is of KIND, not
instances.

⛔⛔ **the falsifier recorded at the field, so nobody adds a `bool`:**
`Block::velocity` means **displacement** (defines the previous pose, selects a
ledge carrier) *and* **surface drag** at once. A belt authored as
`Block { velocity: drag }` would be picked as a ledge carrier and handed a
previous pose it never occupied. ⇒ **split into `displacement` + `surface_drag`
BEFORE any new `BlockKind` or authoring field.**

⚠ **reopen only for a real kinematic customer.** ⇒ two open deletion candidates,
both needing a product call rather than a worker's: `MovingPlatformMotionSpec::Path`
and `DamageVolume.path_id` have **zero authored instances** — the path road is
code-only for both geometry consumers — and `EnemySpawn.path_id`'s inert LDtk
`fieldDef` still sits on 184 instances.

- ⏸ **D116 — Ambition multiplayer/multi-view first slice. RESTING: M2's presentation half CLOSED, its production-composition half DEFERRED.**

Design: [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
and [`game/multiplayer.md`](game/multiplayer.md). Execution detail archived as
evidence:
[`../archive/planning-superseded/2026-08-15/d116-multiview-first-slice.md`](../archive/planning-superseded/2026-08-15/d116-multiview-first-slice.md).

⛔ **do not say "M2 is complete" — it is half done.**

✔ **CLOSED — presentation/projection.** An assembled-host fixture proves per-view
association and viewport application, and **both** `PresentsView` writers that
took `views.iter().next()` are fixed; they now refuse loudly rather than guess.

▢ **DEFERRED — production two-view composition and layout.** Production spawns
**one** camera and publishes **one** screen rectangle to every view **by
construction** (`publish_camera_viewport` projects the single
`ResolvedGameplayPresentation`, a fact about the physical screen). ⚠ M2's own plan
also names **HUD ownership and input routing**, untouched here.

⚠ **three process-globals a split host will owe an answer for:**
`sync_parallax_layers`'s `.single()` (silently stops the backdrop in BOTH views),
`MainCameraEntity` as last-writer-wins, and portal camera continuity.

⛔ **do not expand into networking**, and do not open an M3 on presentation. The
deferred half is gated on a real product need for a second view.

- ✔ **D126 — CLOSED 2026-08-14. Three items resolved, the fourth moved to D115.**

⭐ **all three were the same shape wearing different clothes: something DECLARED a
capability that nothing consumed, and in every case the honest answer was a
deletion or a report rather than a wire.**

1. **The resolver's block-order dependence was an INFEASIBLE-CONSTRAINT problem,
   not an ordering one** — and the proposed "sort by penetration depth" fix was
   rejected because a deterministic wrong answer turns the red test green while
   concealing the physics. `resolve_axis_repair` now separates feasible contacts
   (a legal interval, resolved order-independently) from infeasible ones
   (`AxisConstraintConflict` on `FrameEvents`), and **nothing reads the conflict**:
   damage, death, respawn and forced displacement are Ambition policy. ⭐ the
   derivation collapsed cleanly — `strict_intersects` admits only non-zero
   corrections, so claims in one direction are ALWAYS feasible and claims in both
   are NEVER feasible. No clamp, no tie-break, no epsilon.
2. **`step_kinematic` deleted** — 0 production callers, 26 test invocations all
   inside its own test file, against a live kernel used from 31 files in 10
   crates. It read as live because of 13 stale comment lines in 11 files.
3. **`ActorControlFrame::drop_through` deleted** — no brain ever set it, and it
   could not have been wired without forking a rule, because the gesture is
   derived at the consumer and `InputState` carries no boolean for it.

⇒ **item 4 moved to D115**, where it belongs: a moving platform cannot be authored
one-way because `as_collision_block` hardcodes a blink kind. ⛔ it is **not a
`bool` away** — `one_way_landing_from_previous_feet` compares a PREVIOUS feet
coordinate against a CURRENT face, which is sound for static geometry and a MIXED
FRAME for geometry that moves, so a rising elevator would steal a landing off a
stale feet line. `MovingPlatformState` already carries `previous_aabb()` for
exactly that hazard, and **that question must be answered before the field
exists**. Cost: a field on `MovingPlatformSpec` and `MovingPlatformState` (5
constructors), which is serde-derived rollback snapshot state ⇒ a schema
re-baseline, plus a new LDtk `field_bool`; `MovingPlatformState` is referenced
from 8 crates.

⛔⛔ **one tooling footgun found here and worth keeping:**
`scripts/rollback_codec_shape.py` skips any path containing `/.claude/`, so run
from a worktree agent it sees **zero** codec files and `--record` would **BLANK
the baseline** rather than fail. ⇒ **record baselines from the MAIN tree only.**

Case file: [`../archive/planning-superseded/2026-08-14/d126-resolve-order-and-uncalled-capabilities.md`](../archive/planning-superseded/2026-08-14/d126-resolve-order-and-uncalled-capabilities.md).

- ◐ **D146 — THE SMASH CONTROLLER, AND DASH LEAVING THE VOCABULARY. (Jon,
  2026-08-16, three asks in one message + one follow-up)**

Jon, verbatim: *"Another thing to note is I don't think the special button is
mapped to a game pad for smash. My preferred smash layout for a xbox controller
is a=normal, x=special, b=jump, y=grab (we don't have grab yet), left trigger is
shield. The rest of the bindings are normal I think. Now that each character has
an up-b, I think we can likely also remove everyone's ability to dash in smash.
Dash should be an ability for ambition, it doesn't map into a smash vocabulary.
We may need to give everyone extra height for their double jump to compensate."*

And the follow-up: *"Well, B=jump is the way I like my smash controller, It's
probably non standard. Will need to have control profiles eventually."*

⭐⭐ **JON'S RULING ON WHICH WAY THE AUTHORING POINTS (2026-08-16, mid-slice-2,
and it OVERTURNS a recommendation I had already written into this row).**

Jon, verbatim: *"I think MaryO probably should tumble. The issue is that the
artist needs to author how she does that, similar to how Mario does tumble in
smash ultimate. The real difference is that in a real smash game each character
is authored individually exactly for that game. Which is why my thinking is
going from the character pointing towards the game rather than vice versa.
Otherwise the game is overriding author facts. The trick here is that our
characters happen to behave pretty well in both the ambition style game and the
smash style game, and I also want to be economical and reuse some of the artwork
where I can. If we were doing this super professionally, each game would have
their own artwork specifically authored and with specific information for only
the abilities that happen in that game. We're eventually going to need to offer
her the ability to grab, but she's never going to be able to grab in her actual
game — but all of those grab details should be on the authoring side not the
game side."*

⛔ **I had recommended the opposite** (a per-seat body override on the roster,
"the exception lives with the invitation"). It was wrong, and the repo says so:
**D144 already points character → vocabulary.** Mary-O's sixteen smash moves live
in `game/ambition_demo_mary_o/src/smash_moveset.rs` — HER crate, authored by her,
unreachable at home because her catalog row omits `attack`. Her own file states
the principle: *"a move table is what the attack IS; the ability is whether this
body may attack at all."* `MatchBody` pointing the other way was the inconsistent
thing.

⭐ **THE TEST THAT SETTLES WHO OWNS A NUMBER:**

```text
IDENTICAL repetition is CEREMONY  -> centralize it   (the room's physics)
DIFFERING repetition is CONTENT   -> author it       (the fighter's identity)
```

The six `MatchBody` numbers were the SAME fourteen times — the room is not
asserting anything about Mary-O, it is saying what happens in this room, so
centralizing was right. Gravity, fall speed, weight, air jumps, HOW SHE TUMBLES
would be DIFFERENT fourteen times. That is content, and content belongs to
whoever draws it.

| what | owner | why |
|---|---|---|
| grab / tumble / get-up / tech — frames, geometry, feel | **the character**, against the VOCABULARY | only its author can draw it, and it never names a game |
| gravity, fall speed, weight, air jumps | **the character**, as its FIGHTER self | differs per fighter — that IS the identity |
| tumble threshold, air-dodge window, jump squat, no recoil | **the room** (`MatchBody`) | identical for everyone; the venue's physics |
| where this fighter sits against THIS cast | **the game** | relative — uncomputable in a file that cannot see the roster |

⛔ **the line: a game may RANK its cast; it may not STATE FACTS about them.**
The existing `knockback_weight` spread in `install_smash_content` (v2 0.85,
George 1.35, v3 the 1.0 reference) is the good version of the last row and the
only thing that belongs on the game side. Jon's *"overriding author facts"* names
exactly the failure mode a per-seat body override would have become.

⚠ **economy is not a departure from this.** A character's fighter self MAY reuse
its platformer sheets — the clip fallback chain already does (`smash_forward`
settles for `attack_side`, then `attack`, then `slash`, then `idle`), so an
unauthored fighter frame costs a move its picture and never its gameplay. The
gaps stay playable while the art catches up.

⭐⭐ **AND THE FOLLOW-UP: THE LAST ROW IS DEFERRED ON PURPOSE. DO NOT RE-OPEN IT
AS THOUGH IT WERE UNEXAMINED.**

Jon, verbatim, on the *"where this fighter sits against this cast"* row: *"I do
actually think that the knockback and character weight does belong on the
character authoring side and not on the game side still. The authoring format of
the character can give it a whole bunch of properties and **it's the game's
prerogative if it wants to choose to use it or not.** But maybe this whole thing
is just a big smell and there's a better compositional way to handle it. **Maybe
we should shove the actual decision on how to do this for now as long as the seam
isn't too difficult to maintain or hard to restitch if we decide to do a
refactor.** … the correct move if you're actually making a single game is to put
it all in the author side on the character and then you balance the characters,
because the pool of characters that you're inserted into the game is the cast —
the game itself just imports them, and runs its logic on them. But this weird
we're-using-the-same-character-in-multiple-games really makes the boundary fuzzy
and difficult to reason about how the correct compositionality should be
implemented."*

⭐ **OFFER / CONSUME beats OVERRIDE, and it dissolves the table row above.** A
character DECLARES a pile of properties; a ruleset READS the subset it cares
about. Then nobody overrides anything — weight stops being something the game
does TO George and becomes something George SAYS about himself that a fighting
ruleset happens to read and a platformer ignores.
⭐ **the refinement that holds it up under balance pressure: the CHARACTER
authors the PROPERTY, the RULESET owns the FUNCTION from property to effect.**
George says he is heavy; the smash ruleset decides what heaviness DOES. Balancing
is then tuning the function and choosing the cast — never rewriting a character.
The cast-relative reference frame (George's 1.35 against v3's 1.0) is the only
part that needed the game, and it dissolves too once the property is stated
against a FIXED reference body rather than against whoever is on the grid today.
⚠ **the genuinely fuzzy residue is hitbox/hurtbox GEOMETRY**, where one body
needs different answers per genre. Offer/consume covers it — author both, each
ruleset reads what it needs — but it is unproven and grabs/techs/hurtboxes are
not authored yet. **One data point is not a shape.**

⛔ **WHAT IS OWED WHILE THIS IS DEFERRED — restitch cost, and only that.** Jon's
condition was *"as long as the seam isn't too difficult to maintain or hard to
restitch."* The invariant to hold: **every game-adjusts-a-character edit goes
through ONE NAMED COMPOSITION SEAM, never a reach-in.** Then the eventual
refactor moves one function instead of hunting call sites.

| adjustment | form today | restitch cost |
|---|---|---|
| abilities | `effective_abilities` — stated once | cheap |
| body | `MatchRules::body_over` — stated once | cheap |
| `knockback_weight` | `install_smash_content` MUTATES `definition.vitals` in a loop | ⛔ **a reach-in** |

▢ normalize the third onto the same seam as the other two. **No direction is
implied and no decision is taken by doing so** — it is the shape that makes
either answer a one-edit change later.

⭐⭐ **THE SHARPENED VERSION — "DEFER THE UNIVERSAL CHOICE, BUT NOT THE
BOUNDARY."** (GPT's framing, which Jon endorsed: *"I agree with them"*, 2026-08-16.)

The instruction, verbatim: *"Keep the current D146 work moving. **Do not design
the final universal character/game composition model from one weight customer.**
But **eliminate the registration-time reach-in.** Put Smash's interpretation of
character-authored data behind **one pure named preparation/projection seam.**
Treat character authoring and ruleset specificity as **orthogonal**: a future
`SmashFighterFacet` can be **authored with the character while being owned
semantically by the Smash capability**. Shared properties should only migrate
into the common character/body schema **after multiple real consumers prove they
are actually shared.**"*

⭐ **ORTHOGONALITY is the idea neither earlier framing had.** The argument had
been running as one axis — *does this live on the character or on the game* —
and the answer is that those are two independent questions. WHERE a fact is
authored (with the character, in its own crate) and WHO OWNS ITS MEANING (the
ruleset whose vocabulary it speaks) can differ, and normally should.

⚠ **this is not speculative — D144 already built one.** Mary-O's smash table is
in HER crate (`game/ambition_demo_mary_o/src/smash_moveset.rs`) and speaks
SMASH's vocabulary, unreachable at home because her ability row omits `attack`.
That is a facet: authored with the character, owned semantically by the smash
capability. The pattern exists; what is missing is the NAME and the projection
seam. `MatchAbilities` / `MatchBody` are the ruleset-owned half of the same idea.

▢ **THE ARCHITECTURAL HYPOTHESIS, RECORDED FOR LATER — do NOT build it yet:**

```text
CharacterSpec is NOT "every mechanical truth about this person".

CharacterSpec/package
    = a FEDERATED COLLECTION OF AUTHORED FACETS.

A game/ruleset CONSUMES the facets it understands
    to prepare a body/role for that experience.
```

⭐ this is what dissolves the awkwardness of one character appearing in several
games: a character does not carry a union of every game's needs, and no game
overrides an author fact — each ruleset simply reads the facets it speaks and
ignores the rest. An unauthored facet is a NAMED GAP (see the tumble ruling
above), not a silent default.

⛔ **the migration rule, and it is a brake**: a property moves into the COMMON
character/body schema only after **MULTIPLE REAL CONSUMERS** prove it is shared.
One customer is a facet. ⚠ so `knockback_weight` gets a seam, NOT a schema.

**THREE ITEMS, in the order they should be done. Everything below is MEASURED,
not assumed — the reading was done 2026-08-16 before any of it was written.**

**1 ✔ DASH OUT OF THE SMASH KIT — CLOSED 2026-08-16 (`6db8cab2c`, `a7b5ab681`,
`f4210ba19`, `c0208b21b`). NO COMPENSATING NUMBER: measured, nothing became
unreachable.**
It was not one line. ⛔⛔ **the kernel filled the shared dodge/dash buffer only
for `abilities.dash`, so deleting `dash: true` alone would have deleted the
DODGE from all fourteen fighters in silence** — `apply_dodge` returns on
`buffer_dash <= 0.0` and nothing would have filled it. `apply_intent` now gates
on `dash || dodge` (the same question `movement_actions` already asks to earn
the slot) and the field is `buffer_burst`.
⭐ **the jump measurement came back NO, and the reason is better than the
answer.** `removing_the_dash_from_a_dodging_kit_changes_no_reach` drives the
real kernel through `probe_recovery` on a smash-shaped stage: furthest offstage
recovery is IDENTICAL with the dash bit on and off at every height (170/150/140
px by drift+jump; 180/150/130 with a burst press), because dodge outranks dash
on the shared press and a fighter authors an air-dodge window — airborne, its
press was ALREADY an air dodge, so **the dash bit was dead weight in that kit**.
Poison column (dash, NO dodge) reaches 370/330/280, so the instrument can see a
dash. And the stage is ONE contiguous 480px platform: ground traversal has no
gap to clear.
⭐ the CPU's `smash_dash_to_close` was already locomotion (full throttle) with
an optional `dash_pressed` riding along; the press is gone and the verb is now
`SpecificAction::Sprint` / `sprint_to_close` / `smash_sprint_to_close`, because
closing distance is not a capability. Nothing in the sixteen-press repertoire or
any authored `MoveSpec` read `AbilitySet::dash`.

**1b ✔ THE STAGE LEVELS ABILITIES AND NOW SUPPLIES THE BODY THEY RUN ON —
CLOSED 2026-08-16 (`9817eb949`, `205e52a5e`, `6a74247b5`, `441a0b7cc`).**
Found while doing item 1: only the demo's own three characters authored
`movement_tuning`, so an airborne burst press on everybody else resolved to
nothing at all once `dash` left the kit. ⛔ **measured on the COMPOSED HOST it
was worse than eleven of fourteen: it was TWELVE**, because two of those three
ids are stand-ins the host drops for the real lineage — `player_robot_v3` fought
with the exploration protagonist's 110 px/s melee recoil and no air dodge.
⭐ `MatchBody` (core, beside `MatchAbilities`) is the six numbers a MODE owns —
`slash_recoil`, `jump_squat_time`, the air-dodge window/speed/endlag, the tumble
floor — and `over` composes them onto whatever body a fighter brought;
`MatchParticipantRoster::fighter_body` carries it and `MatchRules::body_over`
states the composition once.
⛔⛔ **a whole `MovementTuning` was tried FIRST and was wrong**: it spreads
`..DEFAULT_TUNING`, so it states every field whether or not its author had an
opinion, and `the_puppy_slug_forced_onto_the_stage_keeps_the_body_it_authored`
caught it in one run — the crawler's authored 80 px/s became the engine's 270.
That is the trap `MatchAbilities` already names on the grant side.
▢ **STILL DEAD — and ⛔ THESE WERE FILED AS "PRODUCT CALLS" AND THAT WAS WRONG.**
Jon, on Mary-O's exemption: *"I think MaryO probably should tumble. The issue is
that the artist needs to author how she does that."* ⇒ **nobody ever decided she
should not; the animation simply does not exist.** A decision and a missing asset
read IDENTICALLY in the code today, and they must not: **an exemption list is a
TODO LIST**, and a granted capability a character has no content for owes a NAMED
GAP WITH AN OWNER, not a quietly different tuning number.
* ▢ **Mary-O: author a tumble, a get-up and an air jump for her FIGHTER self**
  (her own crate, beside `smash_moveset.rs`). She authors `air_jumps: 0` for her
  SMB1 convergence at home; her fighter self wants one, the way Ultimate's Mario
  has one. `air_jumps` is per-fighter in the genre and is NOT a mode's number.
* ▢ **Sanic moves by `SurfaceMomentum`**, which has no `AxisManeuverState`, so no
  stage CAN give him an evade window, a parry or a tumble; `perception_body_for`
  reads `AxisSweptMotion::default()` for him and is right to. ⚠ this one is a
  genuine ENGINE gap, not a missing asset — the other motion model has no seat
  for the state these verbs live in.
▢ **AND A LEVELLED STAGE WHERE THIRTEEN BODIES ARE FLOATIER THAN THE
FOURTEENTH.** The deleted per-character block was ALSO declaring those three
PLAYER-GRADE (`..DEFAULT_TUNING`: gravity 2500, run accel 5200) where a seat
that authors nothing takes `BodyMovementTuning::BASELINE`, the generic ACTOR
body (gravity 1450, run accel 650). It is stated explicitly on the three now, so
nothing moved.
⭐ **JON'S RULING RESOLVES THIS, AND NOT THE WAY IT WAS FILED.** It was filed as
*"which base a platform fighter uses is the decision left"* — i.e. pick one and
level it. It is not a levelling decision at all: gravity, fall speed, weight and
air jumps DIFFER per fighter, so by the ceremony/content test above they are
**fourteen small authored facts nobody has written yet**. Each fighter authors a
FIGHTER BODY beside the fighter moveset it already authors. ⛔ the eleven are not
on a wrong base by choice — they are on the wandering-ENEMY baseline by default,
which is nobody's design.

▢ **AND: smash-correct dodging should eventually come off the SHIELD button,
not the burst button.** In the genre a dodge is shield + direction. Recorded,
not done — it belongs with item 2/3 below.
▢ minor: `resolve_dash` (`affordances/resolvers.rs`) still labels the grounded
prompt "Dash" for a body that now rolls; it reads `is_aerial` and never the
ability set. HUD naming, not behaviour.

**2 ✔ SHIELD IS ITS OWN SEMANTIC ACTION — CLOSED 2026-08-16.** Jon's three
criteria hold: *"Shield input -> can hold/release shield. Special input ->
activates authored special behavior. One cannot accidentally masquerade as the
other."*
⛔⛔ **THE BLAST RADIUS IN THIS ROW WAS WRONG, and the probe is what said so.** It
claimed a human smash fighter could never shield. **A SMASH SEAT CARRIES NO
`PlayerEntity`** — `realize_seat` builds every seat, human and CPU, from
`EnemyActorBundle`, and smash declares `InitialBodyPolicy::NoInitialBody` so no
home avatar exists to adopt. `gate_worn_player_control` is `With<PlayerEntity>`
and therefore never ran on a fighter at all: `a_smash_fighters_shield_input_…`
was written expecting red and came back GREEN on the unmodified tree. ⭐ **the
lesson is the general one — a gate's QUERY FILTER is the blast radius, and it is
one `world.get::<Marker>()` away from being measured rather than reasoned.**
⛔ **the defect was real, on Ambition's own player body.** `gate_worn_player_control`
cleared `shield_held` unless `ActionSet.special == Special("bubble_shield")`, so
any PERSONA a `PlayerEntity` wears that owns `AbilitySet::shield` alongside an
ordinary special lost its guard every frame —
`the_shield_verb_follows_the_ability_not_the_special` fails on the old policy and
passes on the new one.
⭐ the policy moved to where every other slot's already lives:
`resolve_control_slots`' `ControlSlot::Shield` arm now mirrors Attack — absent
slot strips the verb, held item keeps it (shield+attack is the throw gesture),
technique routes it. The kernel's `resolve_shield` still owns the rest, and
`sustain_bubble_shield` is untouched, so a special MAY still raise a guard; it is
simply no longer the only route any body has.
⭐ **renamed, and the reason is stated at the type.** `ControlSlot::QuickAction`
→ `ControlSlot::Shield` and `Platformer2dInputActionMonolith::QuickAction` →
`::Shield` (semantic id `quick_action` → `shield`, preset key
`ActionKeys::quick_action` → `shield`). Measured first: the slot's ONLY occupant
anywhere in the workspace is the shield, and every other slot is already named
for its default action while still hosting techniques. `Modifier`/`Utility` keep
generic names because they genuinely carry more than one meaning.
⭐ **the settings-key consequence is handled, not left to be discovered.**
`ControlSettings::migrate_renamed_actions` (run from `clamp_all`, which every load
path calls) rewrites a stored `"QuickAction"` override to `"Shield"` and collapses
a file holding both spellings; `a_stored_remap_survives_the_shield_action_rename`
pins it.
⭐ **the CPU asks for Shield semantically.** `tick_smash`'s reactive block was
writing `shield_held` by hand beside an unused `SpecificAction::Shield` — two
producers, and the semantic action had none. It commits the action now.
⚠ **the facing is NOT the action's here**: `emit_inputs` faces the target
unconditionally, and letting it do so overrode the footsies weave and pinned both
fighters in a corner (`flying_pca_vs_grounded_robot_is_non_degenerate`, red on the
first attempt) — reactive defense LAYERS onto a chosen action rather than
replacing it.
⚠ **the shipped smash CPU is `template: Fighter`, not the smash brain**, so its
guard comes from `MovementVerb::Shield`. Worth knowing before tuning smash-brain
defense and expecting the stage to change.
Evidence: `a_smash_fighters_shield_input_raises_and_lowers_their_guard`,
`pressing_special_does_not_raise_a_guard_on_a_fighter_whose_special_is_not_one`,
`holding_shield_raises_a_guard_and_fires_no_authored_move`,
`a_cpu_fighter_raises_a_guard_without_pressing_a_physical_button` (all
`smash_in_the_host`), `the_shield_verb_follows_the_ability_not_the_special`,
`a_held_item_keeps_the_shield_verb_alive_without_the_ability`, and the resolver
matrix, which Shield joined.
▢ **NOW UNBLOCKED, and recorded rather than done: dodging comes off the SHIELD
button.** In the genre shield+direction is a roll, shield on the spot is a spot
dodge, and shield in the air is an air dodge — none of them a separate burst
button, which is where they live here. It needed Shield to be a real action
first; it is one now.

**3 ✔ THE PAD LAYOUT, AS A PROFILE RATHER THAN A DEFAULT — CLOSED 2026-08-16.**
Jon's layout, live on a pad in a real match: **A = normal, X = special, B = jump,
Y = grab (blank, see below), LT = shield.** ⛔ and it is a DECLARATION, never a
preset edit: A=Jump is still Ambition's default and the release gives the pad
back when the experience leaves.
⭐ **the middle term the stack was missing** — `device → game binding profile →
semantic action → rules`. `BindingLayout` (`ambition_input/src/layout.rs`) is a
THIRD layer in `BindingRecipe::build`: base preset, then the GAME's layout, then
the USER's overrides. That order IS the precedence decision — a mode's pad is a
better DEFAULT, not an override of the person holding the controller — pinned by
`a_user_remap_beats_the_modes_layout`.
⛔ **not a `Vec<BindingOverride>`, and the reason is shape as well as
provenance.** An override can only move an action ONTO a control; it can never
say *"under this profile that action has no pad button at all"*, which a
permutation of a FULLY-ASSIGNED pad necessarily has to say.
⭐ **keyed by BUTTON, so two properties fall out by construction**: no button can
fire two actions (a layout written as four ADDITIONS would have left B meaning
Jump AND Blink — the exact hazard `presets.rs` refused when it declined to
double-bind Special), and an action the layout displaces without re-homing ends
up unbound on the pad. Blink, Projectile, Utility and Modifier lose their pad
buttons and keep their keys; none is a fighting-game verb. Menu actions are
exempt from the clear (`is_menu_only`, exhaustive), because MenuSelect
deliberately shares South with Jump.
⭐ **this is the only thing that ever gave gamepad-Special a button.** The pinned
policy is SCOPED rather than weakened: the default pad still declines to
double-bind one, and a layout may claim one —
`the_default_pad_leaves_special_to_a_profile_and_a_profile_can_take_it` asserts
both halves.
⚠ **both left shoulder buttons shield.** "Left trigger" on an Xbox pad names the
ANALOG trigger, which Bevy spells `LeftTrigger2` because it spells the BUMPER
`LeftTrigger`; Shield takes both, which is also what the genre does.
⛔⛔ **Y IS A DECLARED BLANK AND THE DEPENDENCY IS GRAB.** The layout CLAIMS
North and binds nothing, so Projectile does not sit on the button a
fighting-game player reaches for to grab. When a Grab action exists it is one
line in `SMASH_PAD`. Blocked on the vocabulary item below.
⛔ **a smash seat's bindings come from the PARTICIPANT, measured not assumed.**
`realize_seat` spawns bodies with `Brain::Player(slot)` and no `InputMap` at
all; the map lives on the `InputParticipant` entity — slot 0 from
`spawn_primary_input_participant`, slots 1..n from
`seat_input_participants_for_roster` (`gamepad_only()`). So the layout is
carried by `apply_active_binding_layout_to_recipes` into EVERY participant's
recipe, not just the primary, and the settings→recipe sync carries it forward
(without that, opening the options screen mid-match would snap the seat back to
Ambition's pad).
Evidence, all driving a REAL pad through the REAL host chain in a REAL match
(`smash_in_the_host`): `on_the_smash_pad_x_fires_the_fighters_authored_special`,
`on_the_smash_pad_the_left_trigger_raises_a_real_guard`,
`on_the_smash_pad_b_jumps_and_a_attacks`,
`quitting_a_smash_match_gives_the_pad_back`; plus `ambition_input::layout`'s
`every_button_the_smash_layout_claims_drives_exactly_one_verb`,
`the_actions_smash_displaces_lose_the_pad_and_keep_the_keyboard`,
`a_layout_rearranges_gameplay_and_leaves_the_menu_alone`,
`installing_the_smash_layout_does_not_move_the_generic_preset`.
⚠ **a jump on this stage reads −131px**: up is NEGATIVE y here, and the first
draft of the B probe asserted a rise and read 0.00 on a working jump. The probe
is unsigned now — which way up is, is the ROOM's business.
▢ **NOT DONE, deliberately: the remap UX still has no gamepad-Special row.** A
layout is a game's answer; a player who wants Special on a pad in AMBITION still
cannot get one without editing settings by hand (P5).

**Standing items this row touches but does not close:**
* ▢ **the press vocabulary grows past sixteen** — Jon, on the kit census:
  *"16 is the current target, but we will need to do more (trips, grabs, falls,
  techs, etc…)"*. `SMASH_KIT` in `smash_roster_movesets.rs` is the list, and the
  ratchet reads its length, so adding a press raises the bar by itself.
* ◐ **D143** — the stage's `unarmed_melee` still does not reach a kit-less seat.
  Unreachable from the grid now that all fourteen author tables; real for the
  next character seated without one.
* ⊙ **the PCA's own kit** still has no `double_jump`, `fast_fall` or `dodge` as a
  CREATURE — it gets them on the stage from the floor. Whether the automaton
  should have them in its own room is Jon's.
* ⊙ **Sanic's kit is `[RunJump]`** — what a runner's kit should actually be (a
  double jump? a fast fall?) is open. Only the ban is settled: never fly, blink
  or wall climb, in any iteration.
* ▢ **`ambition_demo_smash` carries its own FORK of the moveset authoring
  helpers** (`crate::moveset`, with a `Feel` tag the shared one has no concept
  of). D144 moved the shared copy down to `ambition_characters`; unifying the
  fork is its own change and would expose what the fork hides.

- ▢ **D162 — EVERY BOOT PRINTS FOUR WARNINGS AND NOBODY HAS TRIAGED THEM, which
  is how a log stops being read. (opened 2026-08-17, from capture output)**

⭐ **one is now DISMISSED WITH EVIDENCE, so it never needs investigating again:**

```text
SheetRegistry: 39 target(s) claimed twice with different frame geometry
```

⚠ it reads like an art-corruption alarm (*"the survivor crops with the wrong
grid"*) and it is FINE. Counting the full-res tier offline: **166 targets, FOUR
geometry collisions — `goblin`, `robot`, `sandbag`, `toon`, every one a shared
RIG target**, which the code's own doc calls legitimate. **No character id
collides.** The 39 counts CLAIMS, not targets: 17 characters share `toon`, 18
share `robot`, 9 share `goblin`. ⭐ and the guard that would catch the real case,
`report_shadowed_character_sheets`, is silent because there is nothing to say
rather than because it is dead — the offline count agrees with its silence.

▢ **the other three are unexamined:**

```text
1  the controlled body is TOUCHING loading zone `pirate_cove_entry` (Door) and
   the transition did not fire … A `Door` needs the press
2  GgrsSchedule … hierarchy contains redundant edge(s) — `apply_actor_contact_damage`
   cannot be child of system set `WorldPrep`, longer path exists
3  LDtk validation: level 'sanic_sandbox' world origin (9600, 3000) is not
   aligned to 16px grid
```

⚠ **(1) is a diagnostic firing on an ORDINARY state — in SOME rooms.** The body
spawns at the arrival point; where that point overlaps a `Door`, and a Door
needs a press, "touching and not transitioning" is simply what a fresh spawn
looks like. **Measured across four rooms rather than assumed:**

```text
goblin_cantina_lair  1 warning     pirate_cove       1 warning
intro_wake_room      0             square_arena      0
```

⭐ so it is **room-dependent, not every boot** — my first note said "every boot"
and that was wrong. It is still a WARN that fires for correct behaviour in half
the rooms sampled, which is enough to teach people to skim the log.
⭐⭐ **the reading is now DEMONSTRATED, not inferred (2026-08-17).** Driving the
press against that very zone shows the warning fires first, and the door then
works perfectly:

```text
WARN … TOUCHING `pirate_cove_entry` (Door) … interact buffered = false
capture_scene: pressed KeyF (1 of 1)
room-transition begin seq=1 pirate_cove -> central_hub_complex
room-loaded central_hub_complex
```

⇒ the warning describes **a door nobody has pressed yet**, which is every frame
a player stands in a doorway. ⭐ the fix: say it at DEBUG, or fire only when the
press HAPPENED and the transition still did not.
⚠ this also answered a maintainer report in passing — *"in ambition I can't use
F to go through doors anymore"* does not reproduce on the keyboard preset.

✔ **(2) IS RESOLVED AS WON'T-FIX, WITH THE REASON — traced 2026-08-17.** It is a
redundancy that exists because both memberships are individually correct:

```text
features/mod.rs:646   apply_actor_contact_damage.in_set(WorldPrepSet::ContactDamage)
                      …and the whole tuple carries .in_set(…::WorldPrep)
schedule.rs:150       WorldPrepSet::ContactDamage.in_set(…::WorldPrep)
```

⇒ the system reaches `WorldPrep` two ways — directly via the tuple, and via its
member set — so Bevy drops the shorter edge and says so. **Neither declaration is
wrong**: the tuple-level one is what every other system in that tuple needs, and
the per-system one is the consumer contract (*"a consumer can say 'before bodies
move' without naming this function"*).

⚠ **and `WorldPrepSet::Integrate` is nested identically** (`schedule.rs:140-144`)
with `integrate_sim_bodies` in the same tuple, so the same redundancy exists
there and Bevy names only one of them — the message is a sample, not a census.
⛔ so "fix it" would mean restructuring a `.chain()`ed tuple to exclude two
members from a blanket set, which risks reordering a load-bearing simulation
phase to silence a line that ends *"built successfully, however"*. Not worth it.

⚠ (3) is one authored level off-grid, and `tools/ambition_ldtk_tools` is the only
road that may edit a `.ldtk`.

⛔ **the row is about the LOG, not the four items.** Four standing warnings mean
a new warning arrives into noise, which is the failure mode that matters.

- ▢ **D161 — A LOADING ZONE PRINTS ITS AUTHORING ID AT THE PLAYER, and the same
  frame shows the game already knows how to do it properly. (opened 2026-08-17,
  found by CAPTURE of `intro_wake_room` — the flagship's OPENING room)**

⭐⭐ **the contrast is in ONE frame**, which is what makes this a defect rather
than a taste question:

```text
→ corridor            ← authored prose, player-facing, correct
wake_room_arrival     ← an authoring id, printed under the robot
wake_to…              ← an authoring id, printed CLIPPED at the top-right corner
```

All three are loading-zone labels in `intro_wake_room`. A player's first screen
in Ambition shows two internal identifiers.

**Where it comes from** — `spawn_loading_zone` in
`crates/ambition_render/src/rendering/world.rs`, which branches on activation:

```text
activation == Door   → DoorNameplateSource(zone.id, zone.name, aabb)   proximity-gated
otherwise            → spawn_world_label(… &zone.name …)               UNCONDITIONAL
```

⇒ both roads render **`zone.name`**, and a zone's name is a LEVEL-AUTHORING
identifier, not prose. Nothing anywhere asks whether the string is fit to show a
player.

**Measured population** (parsing every `.ldtk` under `game/ambition_content` and
`game/ambition_map_assets`):

```text
loading zones carrying a name          151
name is snake_case, i.e. an id         130   (86%)
of those, NOT Door ⇒ printed always     19
```

⛔ **CORRECTED 2026-08-17 — I first published these DOUBLED (302 / 260 / 38).**
`game/ambition_content/assets/worlds/` and `game/ambition_map_assets/*/worlds/`
are the SAME worlds, and I counted both. ⚠ the ratio survived the error at 86%,
which is exactly why it read as plausible — **a proportion is not a check on a
total.** Same duplication had already inflated a sign-text count in the same
session; fixing it there and not here is the tell that a per-measurement fix is
not a fix.

⚠ **so this is not one bad level.** It reads as a debug affordance that was
never gated, and the `→ corridor` label in the same room proves the
authored-prose road already exists.

⭐ **and a second room makes the point better than the first.**
`goblin_cantina_lair`, captured the same afternoon, puts all three in one frame:

```text
Goblin Cantina — vault the tables to the chieftain.   ← room title, prose
Fretjaw, Cantina Chieftain                            ← character, prose
goblin_cantina_entry                                  ← the zone, a raw id
```

⇒ two authored strings written with care and one identifier, side by side, in a
room a player reaches early. Whatever this is, it is not a decision anyone made.

⭐⭐ **AND A THIRD CAPTURE CHANGES WHAT THE FIX IS.** `water_world`'s door reads
**"to basement hub"** — prose — and it comes from **the same `zone.name` field**
that rendered `goblin_cantina_entry`. So the renderer is not missing a
player-facing road and the schema is not wrong:

```text
water_world          to basement hub        ← zone.name, authored as prose
goblin_cantina_lair  goblin_cantina_entry   ← zone.name, left as the id
```

⇒ **the field already carries prose wherever an author bothered.** 130 of 151
simply never got one, and the renderer faithfully shows whatever is there.

▢ **so this is mostly CONTENT, plus one guard.** Authoring the missing names is
the work; what stops it recurring is a lint that REFUSES a zone name matching
`^[a-z0-9]+(_[a-z0-9]+)+$`, in the same family as the repo's other authoring
checks. ⚠ a lint is only worth adding if it can go green — 260 rows is a
campaign, so it wants a ratchet (may fall, must not rise) rather than a gate
that is red on day one.

⛔ do not "prettify" the id by swapping underscores for spaces — that
manufactures prose the author never wrote, and `wake_to_raid` has no good
rendering. ⚠ and drawing nothing when the name looks like an id is a REGRESSION
for the doors that legitimately want a label; the answer is to author them.

⛔ **AND A SECOND FINDING I WITHDREW — recorded because the withdrawal is the
lesson.** I first wrote that the clipped `wake_to…` at the top-right proved a
world label could escape the world area. **It does not.** These are WORLD-SPACE
labels, so a label near the edge of the room is partly off-camera by
construction: the very same `→ corridor` label is clipped in the first capture
and fully visible in the second once the camera panned. ⭐ right observation,
wrong conclusion — the two frames were already in hand and would have settled it
before it was written down.

⚠ **a separate, genuinely-authored one, found in the same sweep**: the hub room
`central_hub_main` carries a SIGN whose text is a developer note —
*"LDtk-authored central_hub_complex: hub chunk, doors, and continuous basement"*
— rendered large and centred where `intro_wake_room` shows *"creator's basement
lab"*. ⭐ **it is the only one**, and establishing that took two passes:

```text
134 sign texts (268 across the two asset copies, which are duplicates)
 43 match a "looks like a dev note" pattern
  1 actually is
```

⛔⛔ **the other 42 are an AUTHORED HOUSE STYLE and I nearly filed them.** This
game is about an AI in a lab, and its signage speaks that way on purpose —
`// PERIMETER BREACH — TWO HEADINGS`, `// 'this one isn't on the manifest'`,
`[scanner beam]`, `MAP_WATCHED: low route observed`. A regex looking for `//`,
`[…]` and `SCREAMING_CASE:` finds the voice, not a defect.

⭐ **the discriminator, worth keeping**: a dev note talks about the AUTHORING
ARTIFACT — LDtk, chunks, levels — while diegetic text talks about the FICTION.
*"LDtk-authored central_hub_complex: hub chunk"* fails that test and nothing
else does.

▢ **left for Jon rather than fixed**, for two reasons: replacing it means
WRITING player-facing prose for the hub, which is his voice to choose; and a
`.ldtk` must be edited through `tools/ambition_ldtk_tools`, never by
re-serialising the JSON. Deleting the sign outright is the other option and is
also a content call.

- ✔ **D157 — CLOSED 2026-08-16. MARY-O HAD HER WHOLE SMASH MOVESET IN HER
  PLATFORMER. The ability gate did not exist, and a test that reported it was
  overruled. (Jon, PLAYING)**

Jon: *"So maryo seems to have gotten a bunch of moves from smash in her game, and
its messing things up there. **She should only have the run and jump in her
game.** And **the run should double as the fireball button when she has the
lantern.**"*

⭐⭐ **THE CAUSE: `combat_actions` never read the `AbilitySet`.** It derived the
Attack / Special / Projectile slots from the MOVESET and the `ActionSet` alone —
both of which answer *what the attack is*, never *may this body attack*. D144
attached `mary_o_moveset()` to her three character definitions (correctly: the
crossover grid wants those moves, and D146 ruled a character authors its fighter
self), and from that hour her `abilities: Some([RunJump])` row bought nothing.
Measured on the assembled demo: **twenty-three distinct swings** reachable across
attack / smash / special / pogo. Sanic was the identical construction and the
identical break (`spin_charge` and friends live on his own speedway).

⛔⛔ **AND A TEST CAUGHT IT AND WAS ARGUED AWAY.** `persona_architecture::…peaceful_kit…`
asserted `moveset_len == 0`, went red at 17, and `3d3540546` (D146 slice 4)
rewrote it to assert exact equality with `sanic_moveset()` — i.e. to AGREE with
the 17 — reasoning that *"what keeps that off his own speedway is the ABILITY,
not the table"*. That sentence was an INTENT; nothing implemented it. Three
source comments carried the same false claim (both demos' registration loops,
both `smash_moveset` module docs) and are corrected in place.
⇒ a hand-listed chain pins the FUNCTION, not the WIRING: the split
was written down in four places and executed in none.

Landed: `combat_actions` takes the `AbilitySet` and ceilings the melee family
(Attack + Special) with `abilities.attack`. The table stays attached; the gate is
what changed. ⚠ **Projectile is deliberately NOT under it** — `AbilitySet` has no
ranged capability, a ranged verb is always an explicit authored grant
(`ActionSet.ranged`, or an equipment row's), and folding it under `attack` would
mean the only way to arm Mary-O's fireball was to arm her fists.
`SMASH_FIGHTER_KIT` already grants `attack`, so the crossover grid is untouched
(`smash_roster_movesets` + `smash_in_the_host`, 47/47).

⭐ **Part 2 was already built and is now guarded.** `fire_spark_on_run_press` +
`sync_run_action_scheme` have implemented the classic one-button grammar since
the beacon landed; what was missing was any proof it survives in the ASSEMBLED
app (`power_loop` builds its body by hand and never sees the smash table). It
does: the beacon grants the `ranged` verb, the worn identity becomes
`mary_o_fire`, and the run press throws while the run hold still runs.

Guards, both **seen red on the pre-fix tree** (verified by poisoning the ceiling
and re-running): `ambition_demo_mary_o_app`'s
`mary_o_at_home_can_only_run_and_jump` (sweeps every combat button × every aim
and asserts nothing starts, then asserts the run and the jump still work) and
`the_run_button_throws_a_spark_only_while_she_wears_the_lantern`; and
`ambition_demo_sanic_app`'s
`the_demo_body_cannot_trigger_a_single_move_from_its_own_smash_table` (which also
proves the spin-dash TECHNIQUE keeps its Attack button while the repertoire
behind it stays unreachable).

- ✔ **D156 — CLOSED 2026-08-16. THE PATENT CLERK FACED BACKWARDS, AND SO DID
  CARL. The facing was authored THREE TIMES and read ZERO. (Jon, PLAYING)**

Jon: *"Patent clerk faces backwards. Something is interpreting his authored
direction incorrectly. I thought we authored his facing in his metadata? Is that
not being read correctly? Or is it not there?"* Then, after measurement:
*"Patent clerk is facing west, add in that XML, fix his generator and regenerate
him."* Confirmed in game afterwards: *"They both are currently facing the right
way."*

⭐⭐ **THE ANSWER: it WAS there, three layers deep, and nothing in Rust read any
of it.** `CharacterSpec.facing` → the rig's `features.facing` → (Emmy only) the
SVG's `data-rig-facing`. All three said `west` for the clerk and were ACCURATE.
`gravity_aware_flip_x` was exactly `facing < 0.0` with no per-character term, so
the engine assumed all ~800 baked sheets were drawn facing +x.
⛔⛔ **and it was a FORK, not a missing feature.** `animate_bosses` has XORed
precisely this term since the mockingbird; the CHARACTER path is the half that
never got it — unifying a fork EXPOSES what it hid.

Landed: `SheetRecord::authored_faces_left` (`#[serde(default)]`, emitted only
when true), `flip_x = gravity_aware_flip_x(..) ^ spec.authored_faces_left()`,
and `data-rig-facing` lifted out of `_validate_noether_view_contract` (which was
Noether-specific BY NAME with a hardcoded `"east"`) onto `CharacterSpec` so every
character declares its own. Commits `8c30de613`/`37ac258b6` (clerk),
`fd4320071` (Carl); renderer submodule `fac948b` → `9b445c5`.

⚠ **A GAP NEARLY SHIPPED: `SpritePackCatalog::to_sheet_record` synthesizes a
record from ATLAS RECTS and cannot know which way pixels point.** Both characters
are in the ultrapack at all four tiers, so each would have been correct from his
own sheet and backwards again from the pack. `try_load_pack_spec_for_target` now
inherits the base manifest's facing, for the same reason it already inherits
`tuning`.

⛔ **THE LATENT HAZARD THIS LEAVES — worth knowing before touching any rig.**
`facing: str = "west"` is the DEFAULT on `CharacterSpec` and predates all of
this; Emmy is the one who explicitly sets `"east"`. So a rig that declares
nothing INHERITS west, and "the rig says west" can mean *nobody set it* rather
than *the artist drew it that way*. It happened to be true for Carl (verified in
game). ⭐ what actually protects the population is
`every_baked_sheet_is_drawn_pointing_where_its_body_faces`, which asserts drawn
direction against facing for the WHOLE baked population and pins the declaring
set to exactly these eight manifests — a ninth cannot appear silently.

▢ **still open, small**: the portrait tier declares no facing and was never
checked (the agent was stopped mid-look). Portraits are separate art on a
separate path and both characters read correctly in game, so this is a question,
not a known defect.
▢ **two PRE-EXISTING rig-validator failures**, confirmed at HEAD with
byte-identical parts and unrelated to this: Carl's canonical paint-slice order is
out of order, and Noether's rig names `head_base`/`head_features` where
`validate_one` requires `head`/`torso`. The `build` path both regens use is fine;
only `validate` is red.

- ✔ **D158 — CLOSED 2026-08-17. TWO TAUNTING CPUs PRINTED THROUGH EACH
  OTHER: stacking separated the OFFSETS and nothing separated the LINES.**

Seen in the 1200-tick frame of a real two-CPU match: THREE taunt lines, two
separated correctly (~45px) and a third printed straight THROUGH another
bubble's text.

⛔ **the same-frame hypothesis was REFUTED by probe.** Three arrivals in one
frame already came out of `make_room_for_pending_speech_bubble` at `0 / 28 / 56`.
The pending queue was never the problem.

⭐ **`stack_offset` is measured from each SPEAKER'S OWN HEAD, and the two
speakers were not at the same height.** Traced live out of the photographed
match: taunts anchor at `y = 225.44` from the stage floor and at `y = 208.84`
and `y = 196.62` from mid-air. Floor-to-air is 28.8 — one `_STACK_STEP` — so
pushing the older line 28 up from a grounded speaker landed it 0.8 from an
airborne speaker's untouched line. Every offset was distinct and every line was
on top of another. A platform fighter has somebody airborne constantly, so this
is the ordinary geometry, not a corner.

⭐ **the fix: ONE column, swept once, in ELEVATION** (`restack_speech_bubbles`,
`crates/ambition_render/src/fx.rs`). Live entities and this frame's arrivals are
gathered into one list — the two near-identical make-room routines were a FORK,
and each could clear every member of the other population and still collide.
The column sorts by where the text lands (`target_stack_offset - pos.y`), the
lowest line stays at its speaker's head, and every line above is lifted to clear
the one beneath by a step. ⭐ **the arrival is a member, not a privileged
newcomer** — it slots into the middle when that is where it belongs, which is
what fixes the newcomer landing on a line already at the ceiling.

✅ **the fifth bubble RETIRES the oldest line.** `_STACK_MAX` 84 / `_STACK_STEP`
28 is now stated as `_STACK_DEPTH = 4` — the widest supported match — and the
ceiling is measured from the column's own bottom rather than from anyone's head,
so four fighters at four different heights all fit. A line squeezed past the top
ends immediately (alpha is already zero at full age) instead of clamping onto
its neighbour. `SPEECH_BUBBLE_PUSH_FADE_AFTER` is no longer a race being lost:
it is now spent only when a line actually MOVED, and the retired line was within
0.85s of dying anyway.

Guards in `crates/ambition_render/src/fx.rs`, all three seen RED against the
pre-fix algorithm: `speakers_at_different_heights_do_not_print_through_each_other`
(11.4 apart), `a_live_line_and_a_new_line_share_one_column` (behavioural, through
the real `vfx_spawn_messages`, 8.2px apart), and
`a_four_fighter_free_for_all_fits_and_a_fifth_retires_the_oldest_line`
(5 live lines where 4 fit). Re-captured with the D128 invocation at
`--warmup 1200`: three simultaneous taunts, all legible. `cargo check
--workspace --all-targets`, `ambition_render` (129) and `app_it` (412) green.

✔ **two SEPARATE overlaps the same capture shows, neither of them this bug —
both CLOSED as D159 below.** (1) `SPEECH_BUBBLE_STACK_X_RANGE` was 160 while a
taunt renders ~336 world units WIDE, so two lines 202 apart in x did not stack
and DID overlap horizontally: the gate was a point radius and the thing that
collides is a wide box. (2) a fighter's world NAME LABEL printed through the
lowest bubble; the label and the bubble did not know about each other at all.
⇒ **the column described above is gone.** A bubble is now a `WorldLabel` of family
`Speech` and the one ranked placement pass places it against every other world
label — which is where within-family spacing was already happening for the
plates, so this whole mechanism was a duplicate of it.

- ✔ **D160 — CLOSED 2026-08-17. THE PROJECT GATE RAN NO `--lib` TESTS, NOT EVEN
  `ambition_app`'S — it hid two regressions from the same session.**

```text
cargo check -p ambition_app --all-targets   COMPILES lib tests, never RUNS them
cargo test  -p ambition_app --test app_it   runs ONE integration target
⇒ every crate's `--lib` suite is outside the gate, `ambition_app`'s included
```

**What it hid, both landed on `main` and both mine:**
* `ambition_sim_view::control_prompt` — 3 tests, red since `b33525f58` (D157's
  ability gate, three crates away). The fixture handed its body an attack MOVE
  and never said it may attack, so every label it asserted came back `None`.
  ⚠ found only because the D33 relocation agent happened to run that crate.
* `ambition_app::app::versus_fighters` — **D151 step 2 retired the bridge that
  D151 step 1 had just asserted**, one hour apart, and nothing re-ran the guard.

⭐ **`cargo test --workspace --lib` is the unit tier and is FAST** — it found the
second one in a single sweep. ✔ **added to the stated gate in `AGENTS.md`**,
beside the two warnings already there that `-p <one_crate>` is not the gate and
that `ambition_app` is not the whole of it. Both reds are repaired (`ea5ca88df`)
and `--workspace --lib` is green.
⚠ this is the same shape as D134 (the workspace-policy suite nobody ran) and
D137 (the doc ratchet in no gate): **a suite that exists and is not in the gate
is a suite that goes red and stays red.** ⇒ when adding a check, say which
command runs it per-turn, or it is decoration.
⚠ **and BOTH failures were guards that were CORRECT when written** and became
wrong when a rule moved under them — so this is not "someone wrote a bad test",
it is the cost of not re-running the cheap tier.

- ✔ **D159 — CLOSED 2026-08-17. A NAME PLATE PRINTED THROUGH A TAUNT: the
  speech bubble was a FOURTH FAMILY that never joined the one placement pass.**

⭐⭐ **the fix was to JOIN, not to invent, and `label_layout.rs` had already
written the diagnosis** — *"each family used to place itself … Neither could see
the other … and **both passes would correctly report 'no overlaps found'** … it
is the absence of a placement MODEL … every label — whoever spawns it —
participates by carrying a [`WorldLabel`]"*. `WorldLabelFamily` was
`Signage · Fixture · Actor`; a bubble carried no `WorldLabel` at all, so the
nameplate pass and the bubble pass each truthfully reported no overlap about a
frame in which *"George Booul"* sat inside *"Either you are on the stage or you
are not."*

⭐ **RANKED LAST — `Signage · Fixture · Actor · Speech` — on the module's own
test**, which is *which family can move without anything visibly jumping?* A
plate is permanent furniture on a body the eye is using to keep track of who is
who; displacing it makes it hop up and back once per taunt. A bubble is BORN in
motion (it rises through `SPEECH_BUBBLE_BASE_RISE` for its whole 2.2s) and is
gone before the moment is over. There is no reading under which the plate is the
better candidate to absorb the push. The argument is written at the variant.

⚠ **D158's mechanism was SUBSUMED, not separate, and it is deleted.** The pass
already spaces a family against ITSELF — that is how two nameplates avoid each
other — so keeping `restack_speech_bubbles` would have left two systems placing
bubbles, which is exactly how this bug happened. Gone with it:
`speech_bubbles_should_stack`, `advance_speech_bubble_stack_offset`,
`lift_to`/`retire`/`elevation`, `PendingSpeechBubble`, `stack_offset` /
`target_stack_offset`, every `SPEECH_BUBBLE_STACK_*` constant, and
`update_speech_bubble_outlines` (the pass paints outline children through
`WorldLabel::outline_color`). `SpeechBubbleVisual` is now only the line's clock;
`update_speech_bubbles` publishes an anchor and an opacity and writes NEITHER the
transform nor the colour, because the pass is the single writer of both.

✅ **half (1) fell out for free, as predicted: `SPEECH_BUBBLE_STACK_X_RANGE` is
deleted.** It gated stacking on a 160-unit point radius while a taunt renders
~336 wide, so two speakers 202 apart did not stack and did overlap. A pass that
compares the measured BOXES has no radius to be wrong about.

⭐ **and the pass's OWN arithmetic had the same defect, one level down.**
Displacement advanced in a fixed 11px quantum, so clearing one ~22px line of text
cost three steps and a budget of "six steps" bought two lines of clearance, not
six — a four-fighter free-for-all lost a line. A label now lifts to EXACTLY clear
the highest box in its way, and `step_px`/`max_steps` are replaced by one honest
`max_displacement_px` (96.0, sized for four lines of world text in one cluster).
Terminates in at most one pass per already-placed label, and the loop bound says
so rather than trusting the float arithmetic.

**Guards** (`crates/ambition_render/src/fx.rs`, all through the real systems and
read off the `Transform`s the renderer would use):
`a_name_plate_and_a_speech_bubble_do_not_print_through_each_other` — **seen RED
on the pre-fix tree**, both boxes at `Vec2(-649.3, 224.56)`, exact same centre —
and `the_bubble_yields_to_the_name_plate_and_not_the_other_way_round`, which pins
the ranking argument rather than describing it. D158's three scenarios survive
against the mechanism that now answers them, same measured anchors:
`speakers_at_different_heights_do_not_print_through_each_other`,
`a_live_line_and_a_line_born_this_frame_are_placed_together`,
`a_four_fighter_free_for_all_fits_and_a_fifth_never_prints_through`.

⭐ **RE-CAPTURED with D128's invocation at `--warmup 1200`** (`target/d159/match.png`,
1280x720, 16 676 distinct colours — real pixels, not one of the three ways it
renders nothing): **FOUR taunt lines stacked and every one legible, and BOTH
name plates — "George Booul" and "Pirate Admiral" — clear of the column and of
each other.** The frame D158 closed on had three lines with one printed through
another and a plate inside a taunt; this one has four lines, two plates and no
collision anywhere.

✅ `cargo check -p ambition_app --all-targets`, `cargo test --workspace --lib`,
`cargo test -p ambition_app --test app_it` (412) and `ambition_render --lib`
(131) all green.

- ✔ **D155 — CLOSED 2026-08-16. NOBODY GETS LAUNCHED: knockback did not scale
  and an up-tilt did not send anyone up. TWO bugs, both on the shared floor.**

Jon, verbatim: *"when a character is hit up, they actually get knocked up (or I
guess attacks should have an authored launch direction), right now up tilts just
keep the character on the ground. But being able to juggle is going to be
important in smash and in ambition. Also in smash knockback is not really being
applied well. Right now alice is at 1427% and Booul is hitting her, but she's not
going anywhere. We need real knockback and DI. I thought we had it, maybe its
just some parameter tweaks?"*

⭐ **"I thought we had it" was right, and it was not a parameter tweak.** Probed
live in the composed host at 1427%: the magnitude was HEALTHY
(`LaunchSpeed(3269.4)` = `130 + 2.20 × 1427 / 1.0`, exact), the write to the body
was HEALTHY (`pending_launch` carried it and `step_motion` drained it), and DI
was HEALTHY (a CPU victim rotated the launch by `0.308` rad against the declared
`SMASH_DI_MAX_ANGLE` of `0.31`, speed preserved). Two things downstream of all
three were wrong, and each one alone reproduces one half of Jon's report.

**BUG 1 — every authored launch direction in the game was VERTICALLY INVERTED.**
`HitVolume::launch_dir`'s own contract says `(+x = facing, +y = gravity-down)`,
all ~100 authored literals wrote against it (`(0,-1)` = up-launcher, `(0,1)` =
spike), and `player_robot_moveset` already had a RUNNING test asserting the
d-air's `y > 0` means down. `hit_response::knockback_velocity` negated `y` anyway
— to satisfy a doc comment on its own `HitKnockback::launch_dir` that claimed the
opposite. So every up-tilt, up-air and up-smash in the tree spiked its victim
into the floor and every down-air lifted them. Fixed by deleting the negation:
the authored vector IS the local launch, `n * speed`, with only `x` mirrored to
point away from the source. The two disagreeing doc comments now state the
authoring contract, and the two kernel tests that had encoded the inverted
meaning were rewritten.

**BUG 2 — a launch big enough to TUMBLE was resolved as a LANDING on the tick it
was applied.** `accept_external_launch`'s axis-swept arm answered only half of
the question its own doc poses — *"only the model knows whether a launch means
LEAVE THE SURFACE or override the run"* — while the surface-momentum arm answers
both. A launched body kept its stale resting contact into the same step's
`tick_knockdown`, which read `on_ground == true`, called that *touched down while
still tumbling*, and resolved it to a KNOCKDOWN: `kinematics.vel = ZERO` on the
launch tick, prone for `KNOCKDOWN_TIME`. Measured: a standing fighter at 1427%
took a `3269 px/s` launch and moved **zero pixels**. Fixed by clearing
`ground.on_ground` when `launch_into_tumble` returns true — gated on the tumble
answer, not on the launch's direction, so a shove that does not throw you leaves
you planted and every body whose `tumble_speed` is `0.0` (all of Ambition) is
byte-identical.

⚠ **why it hid.** Every existing floor-game test set `on_ground = false` before
launching, so the one situation a fighter is actually in when it gets hit —
standing on the stage — was never stepped. And a hit UNDER the tumble threshold
launched correctly the whole time, so only hits worth watching were deleted.

**Guards, each verified RED on the pre-fix tree before landing:**
`hit_response::launch_direction_tests::an_authored_up_launcher_rises_and_an_authored_spike_drives_down`
and `::the_authored_vector_is_the_local_launch_under_every_gravity` (the spike
half is the poison: an up-only test also passes on a resolver that drops the
sign); `movement::tests::combat_actions::a_launch_that_tumbles_a_standing_body_throws_it_instead_of_knocking_it_down`;
and behaviourally in the live host,
`smash_in_the_host::launched::an_up_tilt_takes_a_grounded_fighter_off_the_floor`
plus `::an_up_tilt_launches_much_further_at_a_high_percent` (3.9px of rise at 0%,
361px at 1427%). The scaling guard measures RISE rather than displacement on
purpose — the pre-fix build launched downward, where the growth term still
produced a big number and the floor absorbed all of it, so a distance guard would
have been satisfied by a victim shoved along the ground. DI is covered at the
whole-launch seam by
`hit_response::launch_direction_tests::opposite_held_directions_steer_one_launch_two_ways`.

⭐ **nothing smash-side was touched.** Both fixes are in
`ambition_platformer2d_core`; Ambition is a customer of the same floor and gets
juggling for free, which is what Jon asked for (*"important in smash AND in
ambition"*).

- ◐ **D147–D154 — THE D140–D145 REVIEW FINDINGS. (external review, 2026-08-16,
  read against the `2381e3a7e` snapshot)** — D147 and D148 CLOSED 2026-08-17;
  both reproduced when probed.

⚠⚠ **PROVENANCE, and it governs how to start each one.** These eight came from a
STRUCTURAL SOURCE REVIEW. The reviewer states plainly that they *"couldn't
independently run Cargo in this review environment"* and *"treated the commits'
reported green suites as evidence rather than rerunning them."* So every finding
below is a READING, not a measurement. ⛔ **Probe each one before fixing it** —
the failing case is written into each row precisely so it can be run first. A
finding that cannot be made to fail is a finding about the reader, not the code.

Jon's disposition, verbatim: *"We also need to take care of the reviewer comments
after this… probably want to document these in case we don't get to them in this
session."* ⇒ **these queue BEHIND D146**, in the order below, which is the
reviewer's own recommended order and not the severity order.

⭐ the reviewer's reason for that order is worth keeping: the first three *"aren't
generic cleanup for its own sake: each one removes a piece of backend ceremony or
hidden state that future agents would otherwise repeatedly have to understand."*

**Explicitly NOT findings** — the reviewer looked and cleared these: the volume of
new moveset lines (*"genuine fighter design… D146 should centralize repertoire
ceremony, not homogenize moves"*), `MatchAbilities::levelled(SMASH_FIGHTER_KIT)`
as policy (Jon's own ruling; only the `None` bridge below is at issue), the new
camera work, and the smash submodule gitlinks (map/SFX/sprite all match their
archived `main` heads; the music renderer being ahead is the known unrelated dirt).

- ✔ **D147 — CLOSED 2026-08-17. GENERIC MATCH ACTIVATION KNEW THE STOCKS
  RULESET'S PRIVATE LATCH. (review finding 1, HIGH, refactor)**

D140 fixed the never-ending second match by inserting
`StocksMatchSettled(false)` inside the GENERIC `activate_the_prepared_match`.
It worked and the dependency pointed the wrong way: the generic activation road
knew that one particular ruleset keeps a process-global boolean latch needing a
reset, and installed the resource even where the ruleset was never composed —
the comment written at the time conceded exactly that.

⭐ **PROBED FIRST, and the coupling is LOAD-BEARING**: with that one line
commented out, `a_second_match_on_the_same_stage_counts_in_and_ends` goes red on
match two with zero winners announced. So this REPLACED it rather than deleting
it.

**Took the reviewer's stronger form.** `StocksMatchSettled` is not a timeless
global — it is *the stocks outcome for match X*. It carries the `MatchInstance`
it is about (the session, and the tick the cast was built on: the two facts that
distinguish one activation from the next), so a new match reads as undecided BY
CONSTRUCTION. Nobody retracts it, nothing is ordered against activation, and
activation names no ruleset. The type moved with its meaning — out of
`ambition_combat::stocks`, which owns the COUNT, into `features::stocks_match`
beside `decide_stocks_match`, which owns the QUESTION and holds the latch's only
two readers; the orphan rule carried the snapshot impl along, and the rollback
registration now sits beside `ActiveMatch`, the receipt the verdict names.

Guards: `the_previous_matchs_verdict_does_not_settle_this_one`,
`a_verdict_from_another_session_does_not_settle_this_match`,
`adopting_a_seat_topology_does_not_un_decide_the_match` (the last is why the key
is the activation's two facts rather than the whole receipt — `seat_topology` is
mutated on a LIVE match). D140's own guards stay green with no activation-side
retraction left in the tree, which is the measurement that the keyed form
carries its behaviour. `797aa480d`.

- ✔ **D148 — CLOSED 2026-08-17. A TEAM VICTORY ANNOUNCED THE LAST SURVIVING
  TEAMMATE INSTEAD OF THE TEAM. (review finding 2, HIGH, a real bug)**

**REPRODUCED, and the guard was seen RED on today's tree.** The card states its
own rule — a team keeps its own name, only a side of ONE is swapped for the
fighter's — and then decided which by COUNTING THE BODIES standing on the
winning side. `take_eliminated_fighters_out_of_play` despawns an eliminated
fighter, so a two-person team that lost a member early has one body at victory.
Measured at the app level through the real shell: a two-versus-two where Red's
seat 1 was knocked out and despawned before Blue was wiped read
`WINNER: Robot v3` against a side called `Red`.

⭐ **body residency recovering match-participant identity**, the error this
campaign keeps hitting. How many fighters a side HAS was frozen when the match
was prepared; how many are STANDING is the match itself. `PreparedMatch::
seats_on_side` is the frozen answer, named by the same `stocks::side_label` the
outcome names sides with.

Guard: `a_team_victory_names_the_team_and_not_its_last_survivor` (its non-vacuity
half asserts seat 1 left play BEFORE the decision — without that it would pass on
the broken code). The solo half of the rule stays asserted by the four-way and
second-match guards, both of which expect a fighter's NAME, so a fix that always
printed the side would go red there. `f0da10217`.

⚠ **the guard was rewritten once, and why is worth keeping**: the first version
let four CPUs fight between the scripted launches, and a hitlag change landing in
another crate flipped the winner. A claim about the WORDING of a card must not
depend on combat tuning — every elimination is now caused by the test on a fixed
schedule.

- ✔ **D149 — MOVE VFX BYPASSES `FxRequest`, SO FOURTEEN MOVESETS HAND-PAIR EVERY
  SOUND. (review finding 3, MED-HIGH, refactor after D146) — LANDED 2026-08-17**

⭐ **the reviewer calls this the most valuable post-D146 cleanup for making
character authoring agent-friendly**, and it is the one that pays D144 back.

The engine already has the abstraction: `FxRequest`, whose own doc says the
simulation writes ONE request for an effect's visual *and paired sound* so the
caller need not remember both, and `process_fx_requests` fans it out to
`VfxMessage::Effect` + `effect_cue(effect)` → `SfxMessage`. But
`dispatch_move_events` takes `MoveEventKind::Vfx` and writes `VfxMessage::Effect`
DIRECTLY, going around the pairing. So every authored burst in the new tables is
a manual `Vfx(x)` + `Sfx("vfx.…x.loop")` couple, and several characters now carry
a test whose entire job is *"every VFX has somebody remembering to put an SFX
beside it"* — infrastructure making content authors remember backend details.

Wanted: the authored thing is a presentation effect (`effect`, `position`,
`scale`) whose sound DEFAULTS to its companion, with `sound = override(…)` and
`sound = silent` as the exceptions. The `.loop` cases are exactly why an override
must exist; they are not a reason to hand-pair the normal ones fourteen times.
⚠ this removes ceremony only — not one fighter mechanic changes.

⭐⭐ **PROBED 2026-08-17. Accurate, and the size + the two blockers are measured.**

**The site**: `dispatch_move_events` (`crates/ambition_combat/src/moveset/mod.rs:1450`)
— the `MoveEventKind::Vfx` arm writes `ambition_vfx::VfxMessage::Effect { pos, fx,
scale }` straight out, while `FxRequest` (`ambition_vfx/src/vfx.rs:299`) exists
one module over and `process_fx_requests` (`ambition_render/src/fx.rs:190`) fans
it to VFX + the effect's own paired cue.

**THE SIZE — 145 authored `sfx(…)` calls across the fourteen tables split three
ways, and only one third is ceremony:**

| | count | disposition |
|---|---|---|
| `vfx.*` with no `.loop` — restates the default pairing | **74** | ▢ DELETABLE, this is the win |
| `vfx.*.loop` — a looping variant of the art's own cue | 20 | ⭐ genuine OVERRIDE, must survive |
| `player.*` / `enemy.*` / `pca.*` — independent sounds | 50 | ⚠ not paired with any vfx; untouched |

⇒ so `sfx = default | override(…) | silent` is exactly the vocabulary the corpus
needs, and the `.loop` rows are the proof the override arm is not speculative.

⛔⛔ **TWO BLOCKERS — this is NOT a one-line swap.**
1. **`FxRequest` carries no presentation source.** `dispatch_move_events` scopes
   its SFX by `ev.presentation_source` (`sfx.write_from(...)` when scoped), and
   `FxRequest` is `{ pos, fx, scale, sfx }`. Routing move VFX through it as-is
   DROPS that scoping. `FxRequest` has to learn the source first — and a new
   message channel owes three things, so check what else reads it.
2. **`process_fx_requests` is installed by `ambition_platformer2d_host`
   (`lib.rs:572`), not by the combat crate.** Any headless fixture that asserts
   on `VfxMessage` after a move event would see nothing once the write becomes an
   `FxRequest`. Inventory those fixtures before switching the arm.

⭐⭐ **CLOSED 2026-08-17, and the half-landing in between was a live feel bug.**
`1e2aa6337` switched the arm and left the 74 restatements standing, so for one
session every burst in four tables played its sound TWICE — individually correct,
jointly a doubled jab, and 412 app tests green throughout. The migration is
complete now rather than reverted: the arm, the override channel and the table
sweep are one change.

- **the 74 were VERIFIED, not assumed.** Every authored `vfx.*` cue was compared
  against `effect_cue(FxId::new(effect))` for the effect it sat beside — the
  row's own `cue` field, read out of the baked FX manifests. **74 equalled it
  exactly and 21 (not 20) were `<cue>.loop`; ZERO differed.** No override was
  wearing a default's name.
- **the override arm is `MoveEventKind::Vfx { sfx: Option<String> }`**, serde-
  defaulted so every authored table stays valid, threaded to `FxRequest.sfx` via
  `FxRequest::with_sfx`. Authoring says it once:
  `vfx_cued(m, at_s, effect, at, scale, cue)`. ⚠ those 21 are not decoration —
  ten shipped rows pack their sound ONLY as `vfx.<family>.<row>.loop` and ship no
  plain row cue at all, so deleting them wholesale would have silenced a held
  field.
- **the guard is `a_paired_burst_is_heard_exactly_once`**
  (`game/ambition_content/src/moveset_sound.rs`), and it was SEEN RED on the
  shipped tables — *"carl_stargan's `jab` writes `vfx.carl_stargan.evidence_ping`
  by hand at 0.05s, and the burst beside it already addresses that cue"*. It runs
  the real `dispatch_move_events` + `process_fx_requests` over every table this
  crate ships, one authored INSTANT at a time, and intersects what the bursts say
  with what the hand-written events say — so "what a burst already addresses" is
  answered by the engine, never by a table transcribed from it. Beside it:
  `a_sustained_burst_keeps_its_looping_cue` (the 21) and
  `a_moves_own_voice_is_not_ceremony` (the 50 independent grunts and chinks).
- **three `every burst is heard` tests were RETIRED** (noether, oiler, carl),
  each with a tombstone naming the guard above. Their whole job was checking that
  a content author had remembered a backend detail, and they could only ever see
  the MISSING half of the pair — never the doubled one.
- **net −74 authored calls; the four tables lose ~95 lines of sound bookkeeping.**
  No `MoveSpec` changed in timing, geometry, damage, launch or gates.

⚠ **the measured residue, deliberately left.** Two moves — Alice's `side_channel`
and the cellular automaton's `garden_growth` — throw the SAME effect at `±x` on
the same frame, so each is now heard twice: two spatialised bursts, two
spatialised sounds. That is a burst COUNT, not a restatement, and the guard says
so in as many words. The `silent` arm of the `default | override | silent`
vocabulary is what would answer it; it is not built, because two mirrored bursts
making two mirrored sounds is not obviously wrong and nothing else in the corpus
wants it. ⚠ and 83 previously-silent bursts across the other tables now make
their row's sound — that is the point of the change, not a side effect.

- ✔ **D150 — A PROJECTILE CHANGES ALLEGIANCE WHEN ITS FIRER DESPAWNS. (review
  finding 4, EXISTING but newly urgent — a D145 follow-up, not a D145 defect) —
  PROBED RED AND CLOSED 2026-08-17**

Not introduced by D145; D145 touched this authority boundary and left the hole.
Allegiance is reconstructed EVERY TICK by querying the firing `Entity`, and the
code states the consequence as intended behaviour: if the firer despawned
mid-flight the shot becomes indiscriminate environmental damage.

In a four-fighter match that reads: fighter fires → loses their final stock →
body despawns → next tick the owner lookup fails → **the shot in flight turns on
everybody, including its own team.** A shot does not become neutral because the
body that fired it stopped being resident.

Wants stable combat attribution carried BY THE SHOT — who fired me, which side
was I on — independent of owner-body lifetime, with reflection able to rewrite it
deliberately. ⭐ this is the same occurrence-vs-entity distinction D125 is
already working through. ⚠ do this before four-player/team matches are showcase
material.

⭐⭐ **REPRODUCED, then closed.** The probe is
`a_shot_outlives_its_firer_without_changing_sides` — two bolts from one seated
fighter, one aimed at a teammate and one at an opponent, the firer despawned
between the first step and the rest. It failed on the line it exists for: *"the
orphaned shot turned on its firer's own team."* Not a reading about the reader.

⭐ **and the shot's PRESENTATION half already had the answer.**
`inherit_projectile_presentation_sources` says it in as many words — *"the bolt
is the emitter … it routinely outlives the body that fired it. So the source is
STAMPED at spawn rather than looked up at impact"* — so the fix is that same
stamp for the other question: `ProjectileAllegiance { faction, team }`, frozen
onto the bolt the first tick it flies, authoritative from then on, registered
rollback state (`projectile.allegiance`) because after a rewind past the firer's
death there is nothing left to re-derive it from.

⚠ two deliberate NOTs. The **grudge** is not frozen — a feud is something the
firer holds now, not a side the shot launched on — and the **faction** stamped is
the authored one, exactly what the owner lookup read, so this changes a LIFETIME
and not a rule. The parry re-own now rewrites the stamp beside the owner handle
(pinned in `reflect_re_owns_the_shot_to_the_parrier_and_reverses_velocity`);
moving only the handle would have left a reflected bolt fighting for whoever
fired it.

⚠ the component lives in `…actor_monolith/src/projectile/allegiance.rs` and NOT
in the model crate: `engine.ambition_projectiles-manifest-deny` forbids
`ambition_combat` / `ambition_characters` there, so the two materializers cannot
stamp it and the stepper freezes it on first sight instead. **The residue**: a
firer who dies in the window between materialization and the bolt's first step
leaves it unstamped forever. Closing that means stamping at birth, which means
either an observer or moving the vocabulary — neither worth building for a
one-tick coincidence, and named here rather than discovered.

⚠ **two guards outside the per-turn gate caught real work, and both were right.**
`engine.runtime-actor-projectile-centralized` rejected the rollback registration
for naming `…actor_monolith::projectile::` directly — the runtime's one door onto
that module is `projectile_schedule.rs`, so the registration goes through the
re-export and the policy stands unedited. And a NEW rollback name reddens **two**
baselines, not one: `scripts/baselines/rollback-schema-baseline.json` (the
absence contract) and `game/ambition_app/tests/rollback_schema_baseline.txt`
(inside `app_it`, the one a per-crate run never sees).

- ✔ **D151 — CLOSED 2026-08-17. `MatchAbilities`' `None → permitted` BRIDGE MADE PERMISSION INTO A
  GRANT. (review finding 5, MEDIUM, retire the bridge)**

Not a new user-visible regression — the pre-D142 code had equivalent migration
behaviour — but D142 promoted it into the new abstraction, where it is now the
thing the API's own documentation invites you to rely on:

```rust
authored.unwrap_or(self.permitted).union(self.granted).intersect(self.permitted)
```

So `MatchAbilities::at_most(kit).apply(None)` MANUFACTURES the whole kit, despite
`at_most` being documented as *"a ceiling only — grant nothing."* The tests
knowingly preserve this as a migration bridge.

⛔ **it gets worse in exactly the use the docs propose for fighter individuality**:
`granted = basic kit`, `permitted = basic kit + wall jump`, meaning "the one
character who authored a wall jump keeps it." An UNAUTHORED character takes the
`None` arm and receives basic + wall jump — mere permission became a grant.

Wanted: `None` means *no authored claim* (a baseline), never *the ruleset
ceiling*. ⚠ **dovetails with D143** (the kit-less seat), and D144's now-explicit
roster is what makes retiring it affordable.

⭐⭐ **PROBED 2026-08-17, AND THE FINDING IS SHARPER THAN THE REVIEW'S: THE
BRIDGE IS LOAD-BEARING RIGHT NOW. DO NOT DELETE IT.**

* **One real `at_most` adopter**: `game/ambition_app/src/app/versus.rs:298`
  (smash uses `levelled`). Its own comment states the intent —
  *"a character keeps what it authored, minus what this duel forbids"*.
* ⛔⛔ **and NEITHER of its two fighters authors any abilities.**
  `versus_fighters::duelists()` builds `arena_duelist_long` and
  `arena_duelist_close` with `with_sheet` / `with_health` / `with_action_set` /
  `with_moveset` / `with_hurtboxes` / `with_voice` — and **no `with_abilities`**.
  So `authored` is `None` for both, and the `None → permitted` arm is what hands
  them their entire kit. **Retiring the bridge naively leaves both duelists with
  NO abilities and breaks the mode outright.**
* ⇒ the stage's stated intent and its actual behaviour already disagree: it says
  a character keeps what it authored, and what these two actually get is the
  ceiling, because they authored nothing.

**THE SAFE ORDER, and it is the reviewer's own condition (*"as explicit
character/body capability declarations become complete"*) made concrete:**
1. ◐ **DONE, pending validation** — the two duelists author
   `VERSUS_FIGHTER_KIT` (`game/ambition_app/src/app/versus_fighters.rs`), the
   kit they were already receiving: `basic()` + `attack` + `fast_fall`, `reset`
   and `interact` riding in from `basic()` exactly as they already did.
   ⭐ **and `versus.rs`'s `at_most(…)` now REFERENCES that constant** instead of
   restating the set inline, so the ceiling and the kit beneath it cannot drift
   into disagreeing — restating it is how the stage became the only thing
   dressing its own cast.
   Guard: `what_the_duelists_author_is_exactly_what_the_bridge_was_handing_them`
   asserts `apply(Some(kit)) == apply(None)` (so the step is provably neutral),
   that the kit can actually fight (so the equality is not comparing two empty
   sets), and that both fighters really carry it now.
2. ✔ **DONE (`d21031fc4`)** — `apply` reads `authored.unwrap_or(AbilitySet::NONE)`.
   An absent claim now gets exactly what the mode GRANTS, and a ceiling grants
   nothing. ⭐ **`levelled` is untouched** (its `granted == permitted`, so the
   smash stage's fourteen fighters are byte-identical); only `at_most` changed,
   and its two fighters were dressed in step 1 for precisely this. Exactly ONE
   test failed — the one that pinned the bridge — and it now pins the rule,
   including the widened-ceiling case the bridge hid: a mode saying
   `permitted ⊃ granted` to let ONE fighter keep a wall jump no longer hands it
   to everybody who stayed silent.
   ⚠ the line's own doc had named today's condition in advance: *"the day it is
   unreachable this line is `unwrap_or(AbilitySet::NONE)`."*
3. ▢ still open —
   ⚠ it must NARROW the body's own kit rather than REPLACE it, which `apply`
   cannot do today because it never receives that kit. That signature is the
   real work, not the `unwrap_or`.
3. ▢ then guard that no seated character anywhere relies on the bridge, so the
   next unauthored fighter fails loudly instead of silently inheriting a ceiling.

⚠ **`MatchBody::over` is the shape to copy** (D146 slice 1b): it takes the base
the fighter brought as a PARAMETER and states only what the mode owns, so its
`authored.unwrap_or(built)` supplies a real base to layer on rather than
manufacturing a claim. That is why the body authority does not have this defect.

- ✔ **D152 — EMPOWERMENT EXPIRY IS A PER-GAME SCHEDULING FOOTGUN. (review finding
  6, move lifecycle ownership into the engine)** — CLOSED 2026-08-17.

Predates the review; smash's respawn protection proved it real. Smash had to add
`run_empowerments` to its own schedule or a two-second invulnerability becomes
PERMANENT invulnerability. Mary-O and Sanic each independently remember to
install the same system.

The current justification is that games differ on whether they want
`apply_contact_harm`. ⭐ **the reviewer accepts that and separates the two
responsibilities**: contact-harm INTERPRETATION is a ruleset choice; ticking,
expiry and releasing projected state are a domain INVARIANT. An author should be
able to write `Empowered::for_seconds(…, 2.0)` without knowing there is a system
elsewhere that must be scheduled or the duration is infinite.

⭐ **PROBED 2026-08-17 — the reviewer's split holds, and the standing defence
does not answer it.**
* **`run_empowerments` is PURELY LIFECYCLE.** Its body ticks `remaining` against
  `WorldTime::scaled_dt`, decides `live`, and releases; a `None` remaining is a
  HELD empowerment with no clock. There is no game-specific policy inside it.
* ⚠ **but the existing doc already declined an engine-owned installation, with
  reasons** (`empowerment.rs`, the `EmpowermentProjectionPlugin` note): Sanic
  installs `run_empowerments` in `GameplayEffects` and deliberately does NOT
  install `apply_contact_harm` (`defeat_badniks` already owns destroy-on-touch,
  and two authorities killing one badnik was the bug it avoided), while Mary-O
  installs both in `FeatureInteraction` with contact harm ordered AFTER expiry.
  It concludes *"what is engine-owned is the INVARIANT, not the order."*
* ⇒ **both are right about different halves, and that is the actionable shape.**
  The declined request was *one installation point for the whole feature*, which
  would have taken away a real per-game choice. The reviewer asks for less: the
  ENGINE installs EXPIRY in a named set; a game that wants `apply_contact_harm`
  orders it against that set. Nothing about contact harm becomes mandatory, and
  forgetting the lifecycle stops being possible.
* ⛔ **five adopters today**, each having remembered: smash, sanic (×2 incl. its
  tests), mary_o (×2 incl. `star.rs`). A sixth game that forgets gets permanent
  invulnerability — which is exactly how smash's respawn protection surfaced it.

⭐ **CLOSED 2026-08-17 — the engine installs the clock in `EmpowermentExpiry`;
the ORDER is still each game's.** `EmpowermentProjectionPlugin` became
`EmpowermentLifecyclePlugin` and now adds `run_empowerments.in_set(
EmpowermentExpiry)` to the sim schedule beside the removal observer it already
owned. All five hand-installations are gone. `apply_contact_harm` stayed
optional and un-installed — Sanic still has none.

⚠ **ONE LITERAL PLACEMENT COULD NOT BE PRESERVED, AND IT IS STRUCTURAL, NOT A
SHORTCUT.** The five sat in THREE mutually exclusive phases — `CombatSet::Settle`
⊂ `Combat` ⊂ `CoreSimulation` (smash), `FeatureInteraction` (mary_o),
`GameplayEffects` (sanic) — and the hosted Ambition app builds all three plugins
at once, each `run_empowerments` gated by its own `in_mode`. One shared set has
one position, and per-game `configure_sets` re-placement is not the escape: three
games nesting one set into three strictly ordered phases is a schedule cycle.
⭐ **so the placement is `GameplayEffects`, the LAST of the three, which is what
makes it ordering-PRESERVING rather than arbitrary.** Every grant site
(`place_respawning_fighters`, `begin_star_power`, `sync_super_form_traits`) is at
or before it, so a grant still gets its projection stamped on the frame it is
made; and every consumer of what `run_empowerments` WRITES —
`Invulnerability` — reads it from `CombatSet::Resolve` inside `CoreSimulation`,
which precedes all three phases, so the grant→read latency is one frame from any
of them. Nothing a body can observe moved.

What each adopter says now:
* **smash** — nothing. Its chain reads no grant, so it needs no edge; the note
  it carried (*"probably an engine-side registration later"*) is what landed.
* **sanic (lib)** — `EmpowermentExpiry.after(sync_super_form_traits)
  .before(emit_sanic_skid_sfx)`, reproducing its exact chain slot. Still no
  `apply_contact_harm`: `defeat_badniks` keeps destroy-on-touch.
* **sanic (tests)** and **mary_o (`star.rs` tests)** — the plugin plus the same
  one-line edge, instead of hand-adding the system.
* **mary_o (lib)** — the two systems it deliberately ordered AFTER expiry
  (`apply_contact_harm`, `play_star_music`) moved out of the `FeatureInteraction`
  chain into `GameplayEffects.after(EmpowermentExpiry)`, so the intent is stated
  rather than implied by adjacency. ⚠ costs nothing: the `HitEvent` contact harm
  writes is consumed by `apply_feature_hit_events` in `CombatSet::Resolve`, which
  precedes BOTH phases.

Guard: `a_timed_empowerment_ends_in_a_composition_that_scheduled_nothing`
(`features::empowerment::tests`) — an app whose ONLY empowerment statement is the
plugin, polling `Empowered` to a 600-tick ceiling rather than counting updates.
Seen RED with the plugin's `add_systems` stubbed out (*"it must be projecting its
reason"*), along with four fixture tests, because the shared fixture now composes
like a game instead of hand-adding `run_empowerments`.

- ✔ **D153 — A MISSING REQUIRED SPRITE PAGE FAILS OPEN. (review finding 7, small)** — CLOSED 2026-08-17.

In the hall-loading repair: `for index in asset.spec.used_pages()` now skips a
page the realization does not have (`error!(…); continue;`), which correctly
avoids waiting on sparse placeholder handles — that was the permanent-spinner fix
and it should stay. But if a spec says page 12 is REQUIRED and the realization
holds five slots, the sequence is now: log an error → omit the required page from
the manifest → the barrier can report Ready → reveal the room with missing
presentation.

Wanted invariant, the inverse of the spinner fix: **a semantically required page
that is absent fails PREPARATION explicitly** — a manifest construction failure or
an explicit failed dependency, not log-and-continue.

⭐ **PROBED 2026-08-17 — accurate, and the exact site is
`add_character_asset` in `game/ambition_app/src/app/world_flow/room_transition_assets.rs:197`.**
It builds the barrier's image manifest, and a `used_pages()` index with no slot
in `asset.pages` logs and `continue`s, so the character is simply not waited on.
⚠ **narrower than it reads, which is why it is still open and still small.**
`pages` is built as `(0..page_count)` in
`character_sprites/assets.rs:643`, so `pages.len() == page_count` always. The arm
is reachable only when the SPEC's frame rects name a page beyond the realized
count — a truncated or mismatched realization, not an ordinary sparse pack.
⚠ `add_character_asset` returns `()`; making this a real failure means threading
a `Result` (or a collected-errors sink) out through the manifest builder. That
signature choice is the work — the `continue` is one line.

⭐ **CLOSED 2026-08-17 — the failure travels IN the manifest, not out of the
builder in a `Result`.** `RoomAssetManifest` gained `unresolved: Vec<String>`,
filled by a `RoomManifestDraft` accumulator that replaces the bare
`BTreeMap` every `add_*` helper threaded; `inspect_room_asset_manifest` counts
each unresolved label as SETTLED-and-FAILED, so the existing
`readiness.failed` → `LoadWorkState::Failed(retryable)` arm refuses the reveal
and the source room stays authoritative. The shape argument, written at the
field: the manifest is already the artifact that reaches both refusers (the
transition contribute/poll pair and startup loading) AND the prefetch cache's
equality key, so a truncated realization can no longer be promoted as equal to
a healthy one; a `Result` would have to be caught at build time, stashed
beside the manifest, and re-joined with it at exactly that decision site.
⭐ **the arm widened by one honest case**: a `used_pages()` index whose slot
EXISTS but holds a default handle is the same defect (required art, nothing to
load), and it is the reachable one — every production realization builds
`pages` as `0..page_count`.
Guards (`ambition_app` lib, `room_transition_assets::tests`):
`a_required_page_the_realization_lacks_refuses_the_room` (seen RED against a
restored log-and-continue: `unresolved` was `[]`) and
`an_unsampled_pack_page_is_neither_waited_on_nor_failed` — the regression guard
for the permanent-spinner fix this sits inside, green under both behaviours by
construction. Fixtures build real specs through `AuthoredSheets::insert_ron` →
`try_load_spec_for_target_authored`, so neither passes vacuously.

- ✔ **D154 — CLOSED 2026-08-17 (`97a5b76ea`). AUTHORED VFX WAS ONLY HALF BODY-LOCAL: POSITION WAS TRANSFORMED,
  ORIENTATION IS NOT. (review finding 8, take with the next directional pass)**

`d6d5810b8` gave `MoveEventKind::Vfx` an `at` and a `scale`, and correctly
transforms `at` through committed facing and the body's gravity frame. But the
message it produces is only `VfxMessage::Effect { pos, fx, scale }`, and the
renderer builds `Transform::from_translation(…)` — no facing/mirror, no rotation,
no body frame.

So a left-facing fighter puts `air_slice` at the correct left-hand POSITION with
the artwork still oriented as though facing right; sideways gravity has the
analogous defect. Invisible for radial effects, visibly wrong for `air_slice`,
`relative_velocity_arrows`, `paired_trajectory`, streaks, and any future
sword/beam.

Wanted: the presentation event carries a small semantic POSE — position,
body-frame orientation, facing/mirror, scale — instead of accreting one isolated
presentation field at a time. ⚠ not urgent, but **fix it before agents author
hundreds more directional effects against the incomplete contract**, because the
cost is in the content written against it, not in the field.

⭐ **PROBED 2026-08-17 — accurate and unchanged.** `VfxMessage::Effect` in
`crates/ambition_vfx/src/vfx.rs:237` is exactly `{ pos, fx, scale }`. No
rotation, no mirror, no body frame. So the authored `at` is transformed into
world space and the ARTWORK is not, and a left-facing fighter's `air_slice`
lands in the right place pointing the wrong way.
⚠ **the D156 facing work is the precedent to copy, not a duplicate of it.** That
one taught the SHEET renderer which way its artwork was drawn
(`authored_faces_left`, XORed into `flip_x`); this is the same question one
layer over, for effect art that has no body to inherit a facing from. Whatever
carries it should be a small semantic POSE on the event — position, body-frame
orientation, mirror, scale — rather than a fourth isolated field.

⭐⭐ **THE SHAPE IS ALREADY HALF-BUILT, and using it is the whole design.**
`build_move_events` (`crates/ambition_combat/src/moveset/mod.rs:~603`) already
derives the authored offset from exactly TWO authorities:

```rust
body_frame.to_world(ae::Vec2::new(at.0 * pb.facing, at.1))
//  ^ the owner's AccelerationFrame        ^ the move's COMMITTED facing
```

⇒ **orientation must come from those same two, never a third.** A pose derived
anywhere else can disagree with the position it decorates, which is the bug
class this repo keeps paying for (a hand-kept reconstruction ledger: one more
row is not a fix).

**The route, end to end — no authored content changes at all:**
1. `MoveEventMessage` (combat, `~line 180`) gains a POSE beside `world_offset`,
   computed in the same expression: the mirror is `pb.facing < 0.0`, the angle is
   the body frame's — `gravity_upright_angle` (`shared_tangle/src/gravity.rs:394`)
   is what the SPRITE renderer already uses, so an effect and the body it hangs
   off take one rule.
2. `FxRequest` carries it (it already learned `source` and `sfx` this session).
3. `VfxMessage::Effect` carries it — today it is exactly `{ pos, fx, scale }`.
4. The renderer applies it: `sprite.flip_x` + a rotation, instead of
   `Transform::from_translation` alone.

⚠ **IDENTITY is the default**, so every non-move caller (hazards, breakables,
projectiles, fireworks) is byte-identical and this cannot double or move
anything — ⛔ unlike D149's swap, which added a side effect and needed the
callers migrated in the same breath
(routing a producer onto a pairing channel DOUBLE-FIRES — guard a COUNT, not a
presence).
⚠ **it IS a deliberate behaviour change for move effects**: a left-facing
fighter's `air_slice` starts pointing left. Radial art is unaffected; nothing
authored today can have compensated, because no mirror existed to compensate
for.

- ✔ **D140 — CLOSED 2026-08-16. A second match never started and never ended:
  "GO!" stayed up and nothing could win. (Jon, REPRODUCIBLE)**

Jon, verbatim: *"sometimes in a 4 player cpu battle, when someone wins it ends I
with 'Go'. Maybe that is a side effect of starting one match, and then doing
another match? I thought we had tests for that. Probably worth strenthening
them. When there is only 1 player alive or 1 team alive for team matches the
time in the game should freeze with 'WINNER: <name>' to show the match is over,
and not let players continue to play after the match ends. COnfirming a test.
cpu vs cpu on a fresh match, got seat 2 wins. Running back and doing another cpu
vs cpu after gets a 3 2 1 go, but the GO stays on the screen for the entire
match, and the match does not end. I can quit to title and then do another match
which does a 3, 2, 1, go, but again the go still appears on the screen, and the
match does not end when there is only 1 player left. On a fresh start, with 4
player cpu free for all, the 3, 2, 1, go happened, and the go disappaeared, and
the game ended when someone won with the screen text confirmation. But then
another cpu vs cpu the go issue appears again, so this is reproducible."*

⭐ **the repro is exact and it is a SEQUENCE, which is why a test missed it**:
match 1 is correct in every shape he tried (2-CPU and 4-CPU free-for-all), and
every guard on this stage plays exactly one match. His own note — *"I thought we
had tests for that"* — was the finding.

**MEASURED, not reasoned** (`the_stage_kills::a_second_match_on_the_same_stage_counts_in_and_ends`,
written red first). The card said this on match one:

```text
  match 1   3 . 2 . 1 . GO! . "seat 1 wins" . GO!      <- the ceremony took the card BACK
  match 2   3 . 2 . 1 . GO! . (nothing, ever)          <- never decided
```

⇒ **TWO defects that met on one `if`**, both now fixed:

1. **`StocksMatchSettled` could not be retracted between matches.** It was
   cleared only by `decide_stocks_match` observing NO active match — *"a match
   that went away un-decides itself"* — and this stage never removes the receipt
   (the versus stage does; activation REPLACES it). So there is no sim tick
   between two matches on which the receipt is absent, match two opened wearing
   match one's verdict, and `decide_stocks_match` returned on its first line
   forever. ⇒ **retracted by ACTIVATION now**, in the same command flush as the
   receipt: a match that is starting has not been decided.
2. **The announce card had two writers and no arbitration.** The GO! card holds
   one beat past the release; a knockout inside that beat overwrote the victory
   card on the next tick. The old guard protected the wrong half (*"do not CLEAR
   the winner's card"*), which also meant that once (1) made the verdict
   permanent, the clear was gated off forever and GO! sat on a live match. ⇒ the
   ceremony now **stops talking the moment the match is decided**, and the clear
   is unconditional.

**And the product rule he stated is built**: the sim clock is requested to `0.0`
while a match is settled and back to `1.0` while one is live and undecided
(`stocks_match::state_the_matchs_pace`) — self-healing, because nothing else
says "full speed" on a CPU-vs-CPU stage, and safe because the sink reduces a
frame's requests by `min` so hitstop still wins. The winner is NAMED: the card
reads `WINNER: Robot v3`, resolving the engine's SIDE ("seat 1") to the
fighter's own `Name` when the side is a side of one, and keeping the team's name
when a team won together (closes D128 item 4).

**The guard is the sequence**: two matches in ONE app, asserting on each that it
counts in `3-2-1-GO!`, that the ceremony does not have the last word, that
exactly one winner is announced, and that no fighter travels more than 8px after
the end. ⛔ a test that builds a fresh app per match cannot fail this, which is
why the existing ones passed.

- ◐ **D143 — the stage's unarmed declaration does not reach the seat. NO LONGER
  REACHABLE FROM THE GRID (2026-08-16), and the plumbing gap stands. (found while
  answering Jon's moveset census)**

⭐ **the four fighters it was about now author sixteen moves each** (D144), so
nothing on the shipped grid depends on the unarmed floor any more. What is still
true is the defect itself: a character that authors no table reaches a seat with
nothing, on a stage that declared a swipe for exactly that case. The next kit-less
character to be seated finds it again.

**MEASURED THREE WAYS, in the shipped host, seating `mary_o` and `npc_alice`
through the real select screen** (`smash_in_the_host::report_what_an_unarmed_
fighter_swings_once_the_stage_has_armed_it`):

```text
  character kit      moves = 0
  live ActorMoveset  all sixteen presses resolve to SILENT
  live CombatKit     innate_melee / innate_ranged / innate_special all None
  the STAGE          DeclaredCombatRules::unarmed_melee  present = true
```

⇒ the declaration EXISTS and the body does not have it. `mary_o`, `sanic`,
`npc_alice` and `npc_bob` are four of the fourteen selectable fighters, and on
the shipped stage they cannot hit anybody.

⛔⛔ **the guard that should have caught this SUPPLIES the missing value itself.**
`smash_roster_movesets::the_match_gives_every_seat_a_kit_that_can_hit` calls
`roster_seeded(.., Some(MeleeActionSpec::Swipe(..)))` — passing the swipe in by
hand, with a comment saying the shipped experience puts it on
`DeclaredCombatRules::unarmed_melee`. So it proves the SEATING half and never
reads the resource, which is the half that is broken. A fixture that
manufactures the value under test cannot fail on its absence.

**Most likely cause, not yet confirmed**: the publisher reads
`rules.as_deref().and_then(|rules| rules.unarmed_melee.clone())` at the moment
START is pressed — on the SELECT route, deliberately *before* the route changes
(*"the roster is inserted BEFORE the route changes, and the order is the whole
correctness argument"*). If the declaration is installed with the GAMEPLAY route,
it is not there yet and `None` is what gets published. ⚠ the probe above reads
the resource AFTER the match is live, which is why it sees `true`; nothing has
yet read it at publish time.

**Next step**: instrument the publisher (or assert on `MatchParticipant::
action_set` in the roster the shipped screen actually publishes, rather than one
a fixture built) and confirm the ordering before changing anything.

⚠ **and the product question rides along, already filed**: whether the peaceful
cast should be armed by the stage at all or be re-authored as fighters is Jon's
(`awaiting-maintainer-decision.md`). This row is the PLUMBING half — the stage
says a thing and the body does not hear it — which is a defect under either
answer.

- ✔ **D144 — CLOSED 2026-08-16. Every selectable fighter has the full sixteen-press
  smash kit. (Jon)**

Jon, verbatim: *"Let's complete the kit for all characters, authoring new moves
when we need to. 16 is the current target, but we will need to do more (trips,
grabs, falls, techs, etc…). We can invent whatever special we want for oni and
goblin. It doesn't have to be fancy we can use generic sfx / vfx, although I think
oni has a bunch of sfx and vfx ready for it."*

**Measured first, and the measurement moved twice.** The census resolves each
press the way a body does (`move_for_directional_verb`, on the MERGED kit) rather
than asking whether a verb key exists — `directional_verb_chain` FALLS BACK, so a
missing forward tilt is not silence, it is the jab again.

⛔ **and asking only ONE POSTURE invented a gap**: George Booul's down-B is a
commanded plunge and is `airborne_only` by design, so probed standing it fell to
his neutral-B and read as missing. He was 16/16 all along. A press is covered when
SOME posture reaches a move of its own.

```text
                        before        after      what was written
  robot v3               12/16        16/16      ftilt + side/up/down-B
  george booul           16/16        16/16      (the census was wrong)
  pirate admiral         15/16        16/16      ftilt
  goblin                 11/16        16/16      ftilt + all four specials
  shadow oni leader      11/16        16/16      ftilt + all four specials
  perfect cellular auto   8/16        16/16      fifteen — it had one move
  mary_o / sanic          0/16        16/16      sixteen each
  npc_alice / npc_bob     0/16        16/16      sixteen each
  oiler/noether/stargan/clerk        16/16       already complete
```

⭐⭐ **the up-B is the half that is not cosmetic.** The goblin, the Oni, the
automaton, both protagonists and both Hall NPCs had NO special at all — on a
platform fighter that is no way back to the stage. Every one of them has a
recovery now.

⚠ **`moveset_authoring` moved down to `ambition_characters`**, for the same
reason `build_actor_moveset` did: it lived in `ambition_content`, so a character
belonging to any other provider could not use it. Mary-O and Sanic are registered
by their own demos, which do not depend on Ambition's content crate — the choice
was a fourth copy of the helpers or one move. ⛔ `ambition_demo_smash` still
carries its own fork (`crate::moveset`, with a `Feel` tag this one has no concept
of); unifying it is its own change.

⛔⛔ **AND IT CHANGES NOTHING IN MARY-O'S OR SANIC'S OWN GAMES.** Both author
`abilities: Some([RunJump])`, which carries no `attack`: a move table is *what the
swing IS* and the ability is *whether this body may swing at all*. Their sixteen
moves are unreachable at home and reach a body the moment a stage GRANTS the verb
(D142's `MatchAbilities::levelled`). That split is what makes "a classic
platformer protagonist on a fighting grid" expressible rather than a
contradiction. ⚠ Sanic's spin dash and transform stay TECHNIQUES and are not
touched.

⚠ **`hall_humanoids` emptied itself**, exactly as its own rule said it would
(*"If one of them grows a moveset or a distinct build, it earns its own file that
day"*): all four left within a week and what survives is the one fact they still
share, the 210 px/s humanoid walk.

⭐⭐ **AND A SPECIAL OWES AN ANSWER IN BOTH POSTURES** (Jon, later the same day):
*"A down-b that has special airborne properties should also have an effect on
ground. Think of bowser down b … Specials can have different effects in different
contexts that should be ok, and makes for a richer smash game, although in most
cases they shouldn't be context dependent."*

⛔ **the mechanism was already in the engine and nothing had used it.**
`directional_verb_chain` puts `special_air_down` ahead of `special_down` in the
airborne chain, so a two-form move is AUTHORED, not engineered. What the rule
found:

```text
  george booul, sanic     air-only down-B  -> pressed grounded, gave the NEUTRAL-B
  nine others             ground-only down-B -> pressed airborne, same fallback
  goblin, sanic, clerk    ground-only NEUTRAL-B -> pressed airborne, SILENT
                          (`special` is the last candidate in the chain)
```

⇒ George gets the literal Bowser shape — a grounded arc, then the same plunge —
Sanic gets a hop into his ball drop, nine fighters get an air form of their down-B,
and the three neutral-Bs become `either_posture`. ⚠ the arcs are `ImpulseMode::Add`,
not `Set`: `lift_speed` is derived from `Set` impulses, and written that way the
hop told the recovery policy that a DOWN-B was a way home. George's own up-B poison
caught it.

**The census asks both postures now** and reports which one failed and what it got
instead (`dspecial/air=nspecial`).

⚠ **three guards changed with the population, and none was weakened**:
Oiler's geyser control said *"the other seat has no way home"* — impossible once
everybody has an up-B, so it now asserts both seats advertise one and they DIFFER;
`the_grid_fighters_that_state_their_own_moves_only_grow`'s control arm asked to be
checked before the silent column emptied and is flipped to assert it IS empty; and
`the_grid_fighters_with_a_real_repertoire_only_grow` is DELETED by its own closing
line (*"P3.24 is DONE — in which case delete it rather than leave it passing"*),
its guard living on in the two above.

⛔ **and one test was passing on a margin nobody had measured.**
`a_respawning_fighter_is_briefly_untouchable` ran 300 `app.update()`s and called it
five seconds; probed, the grant went 1.967s → 0.233s over those 300 updates, so the
SIM advanced 1.73s. It failed because the app got heavier, not because respawn
protection changed. It runs until the grant ends now, with a ceiling.

**The census is a RATCHET now** — `report_the_smash_kit_every_selectable_fighter_has`
fails on a fighter that drops below the full kit, and reads the target from
`SMASH_KIT.len()` so trips/grabs/techs joining the vocabulary raises the bar by
themselves rather than by editing a number.

- ✔ **D145 — CLOSED 2026-08-16. No projectile could hit anybody on the smash
  stage. (Jon, opportunistic)**

Jon, verbatim: *"Another thing to note is that PCA's glider doesn't do any damage
or hit anyone. Not the priority right now, but if you see the issue
opportunistically fix it."*

⛔⛔ **it was never about the glider.** Melee and projectiles asked different
questions about who may be hit, and only one of them knew what a match is:

```text
  melee        targeting::team_allows_damage(attacker_team, victim_team)
  projectile   damage_lands(firer_faction, victim_faction, ..)   <- no team, ever
```

**Measured on the shipped stage**: both seats come back `ActorFaction::Player`
with teams `seat 1` and `seat 2` — which is correct, because a Hall NPC and a demo
protagonist are not enemies of each other outside the match. So melee landed and
every shot was spared as an ally. ⇒ **no projectile from any fighter could hit
anybody**: not the glider, not the admiral's grapeshot, not Oiler's.

⭐ **the fix is one call.** `damage_lands_between` — the team-aware sibling melee
already used — existed, and `StrikeVictim` has carried the victim's `team` the
whole time with the doc *"Outranks faction for 'may this land'"*. This loop was
the one caller that never asked for it. Guarded by a fixture with the poison
inside it: a body on the FIRER'S OWN team, overlapping the same shot, that must
NOT be hit.

- ✔ **D142 — CLOSED 2026-08-16. A match could only ever TAKE verbs away, so no
  stage could promise a fighter anything. (Jon)**

Jon, verbatim: *"Sanic should never have fly, blink, or wall climb in any
iteration. PCA needs double jump, fast fall, and dodge. In fact, in smash all
characters should be sure they are granted the basic smash abilities, but we want
to do this in an elegant way."*

⭐ **the elegant way is that a match has TWO things to say, not one.**
`MatchParticipantRoster::fighter_abilities` carried a single `AbilitySet` that
was intersected, so a mode could forbid and never guarantee. It is a
`MatchAbilities` now:

```text
  granted    every fighter HAS these, whatever its character authored
  permitted  and no fighter has anything OUTSIDE these
  effective = (authored ∪ granted) ∩ permitted
```

⇒ smash declares `MatchAbilities::levelled(SMASH_FIGHTER_KIT)` — granted ==
permitted, one kit for everybody — and versus declares `at_most(..)`, which is
exactly the lone mask it always was. The four-row bridge table in
`effective_abilities` collapses to `rules.apply(authored)`, with the
unauthored-character bridge expressed as a DEFAULT (`unwrap_or(permitted)`)
rather than a branch.

⭐⭐ **it makes a real tension EXPRESSIBLE instead of picking a winner.** Jon's
older compositional ruling is pinned by a test — *"Forcing Puppy Slug into Smash
gives you Puppy Slug … Jump → no jump if its body cannot jump"* — and today's is
its opposite. Both are right about different modes: a match that MANUFACTURES
capabilities is what made a slug jump like a humanoid, and a match that cannot
GUARANTEE them is what sent the PCA onto a platform-fighter stage with no double
jump. The two halves are now two tests (`a_match_cannot_grant_a_verb_the_
character_does_not_have` for `at_most`, `a_levelling_match_hands_every_fighter_
the_kit_it_declares` for `levelled`), and a stage picks in one word.
⚠ **the cost, stated**: a Puppy Slug forced onto the smash grid now jumps.

**What changed in play: one fighter.** Every other seat resolves to the same kit
it had — the robot and the duelists author supersets of the stage's kit, and
twelve of the fourteen author nothing. The PCA gains `double_jump`, `fast_fall`,
`dodge`, `pogo`, `directional_primary` and `ledge_grab`, which is Jon's first
sentence.

⚠ **its own row is deliberately NOT patched.** An earlier pass authored
`ledge_grab: true` onto the PCA to work around the mask; that is removed with the
reason that justified it. A character does not carry verbs to compensate for a
mode — whether the PCA should grab ledges in its OWN room is a separate call.

**Sanic authors `[RunJump]` on BOTH iterations** (base and super), replacing an
earlier `[SaneSubset]` that granted fly, fly_toggle, blink, precision_blink,
wall_jump, wall_cling and wall_climb — three of the four verbs Jon named it
against. The super form is SPEED (its momentum row), not a capability unlock.
⛔ everything that makes the demo Sanic is elsewhere and stays there: the
momentum model rides the loop, and spin dash and the transform are TECHNIQUES.

- ✔ **D141 — CLOSED 2026-08-16. One fighter on the smash grid could not grab a
  ledge, and the ledge lived at home on two who should not have it. (Jon)**

Jon, verbatim: *"ensure that every character in smash is authored with the ledge
grab ability. Note: we need to make sure mary-o and sanic do NOT get this ability
in their games."*

**MEASURED across all fourteen** (`smash_roster_movesets::every_fighter_on_the_smash_grid_can_grab_a_ledge`):
twelve author no kit and take the stage's set verbatim; two author one. Of those
two, `player_robot_v3` says `ledge_grab: true` and the **Perfect Cellular
Automaton did not** — its kit was written for the DUEL ARENA on
`AbilitySet::basic()`, whose answer is `false`. `fighter_abilities` is an
INTERSECTION, so the stage could not give it back: the one fighter on the grid
whose sheet has ten ledge rows drawn for it was the one who could not use them.
⇒ authored `ledge_grab: true` on the PCA.

⚠ **the same read says its `double_jump`, `fast_fall` and `dodge` are still
`basic()`'s answers** — a real recovery handicap on a platform fighter, left
alone because that is a balance call about this character rather than part of
the ask.

⭐ **the two roads are what makes "in smash but not at home" expressible**, and
they are easy to mistake for one:

```text
  its own game    catalog GRANT LIST     -> the session's avatar   (`session/setup`)
  a smash seat    character DEFINITION ∩ the match's mask          (`prepared_match`)
```

Mary-O's row has said `abilities: Some([RunJump])` since her demo landed. **Sanic's
authored nothing**, and a row that authors nothing falls through to
`EditableAbilitySet::default()` — which is `sandbox_all` — so he was carrying
ledge grab, swim, glide, dodge and a bubble shield around his own speedway, with
his control gate resolving Attack and Utility onto spin dash and transform so
nothing on screen said so. ⇒ his row authors `[SaneSubset]`, the engine's named
baseline, which excludes those five BY NAME. ⛔ what a runner's kit should
actually be (no double jump? no blink?) is still open and is not answered here.

Both halves are guarded in one file, because each is one edit away from breaking
the other.

- ✔ **D138 — CLOSED 2026-08-17, CONFIRMED BY JON: *"Oiler fights in his new body
  now."* The row below is the state BEFORE the fix and is kept for its
  reasoning.** Verified against HEAD: `regen_sprites.sh` no longer lists `oiler`
  in `review_cues` — the entry there is now an explicit ⛔ refusal (*"oiler is
  NOT here any more — see `tackon_targets`. His body comes from the direct-SVG
  rig; leaving the review cue in place would have this loop overwrite the rig's
  sheet with the toon render on every full run"*), and `oiler` sits in
  `tackon_targets` instead.
  ⚠ **found while arming the 72h goal, and it is the reason that sweep exists**:
  a stale ▢ row aims an autonomous run at work that is already done.

Jon, verbatim: *"Oiler's sprite is still his python based one not his SVG based
one. I would like to completely move the SVG one. Note the SVG is used as the
portrait in smash, but not for the actual fight. Similar in ambition itself."*

**Current state, read off HEAD.** `regen_sprites.sh` says the split out loud and
on purpose — *"Oiler is the representative direct-SVG rig target. Render only its
portrait product here so full regeneration does not replace the established
gameplay sheet selected by the review config"* — so `portraits oiler` publishes
`oiler_portraits.{png,ron}` from the rig while `oiler` sits in `review_cues`
and keeps shipping the Python-drawn gameplay sheet. The rig itself is real and
tested: `data/characters/oiler/oiler-multiview.svg`,
`targets/characters/rigged/oiler`, `tests/test_oiler_svg_rig.py`.

⭐ **and the swap is at the PUBLISHER, not in either game.** Both games reach the
body through one filename: `character_catalog.ron` binds `npc_oiler` to
`sprites/oiler_spritesheet.png` + `.ron`, and Smash's select screen reaches the
portrait through `oiler_portraits.png`. So nothing in Rust chooses the Python
art — `regen_sprites.sh` decides which renderer writes that one pair of files.
⛔ therefore do not go looking for a game-side binding to flip; there isn't one,
and Jon confirmed the old sprite is still live in HEAD (2026-08-16).

⇒ **next executable action:** make the rigged target's FULL sheet the published
gameplay sheet (`--target oiler` already claims a sheet+portrait bundle), take
`oiler` out of `review_cues` so a full regen cannot restore the Python art, and
delete the carve-out comment rather than editing around it. ⛔ the deletion is
the proof here: two publishers for one character's body is the defect.

⚠ **the three things that will bite, in order:** the rig's frame geometry is not
the review target's, so `body_metrics`/`collision_scale` must be re-derived and
not assumed; D129's clipped-frame report must be read for the new sheet before
it ships; and the quality tiers (`sprites_0_5x/…`) are a separate published
copy — a stale tier reads exactly like a character swap.

**Falsifier (both games, because Jon named both):** a capture of Oiler in a Smash
match and in Ambition draws the SVG body, not merely the SVG portrait — and the
old Python sheet is gone from `$sprites_dir`, not shadowed.

⛔⛔ **AND THE SHEET IS NOT THE LAST COPY — THE ULTRAPACK IS.** Found 2026-08-16
while doing this: after the rig's sheet was installed over the toon one,
`assets/sprite_packs/full/ultrapack.json` still carried `oiler` with
`src: [102, 222]` — the TOON frame size. The four tier atlases are baked from
whatever was in `$sprites_dir` when they were last packed, so a target whose
sheet changed keeps drawing its old art until the packs are rebuilt. ⇒ **any
sheet swap owes three regenerations, not one**: `--target <t>`, then
`regen_visual_quality_variants.sh --target <t>`, then the four ultrapack tiers.
⚠ this is very likely why Jon saw the old sprite after a regen that did replace
the sheet.

✔✔ **BOTH HALVES DONE 2026-08-16.** The body swap landed (`69eee645f`) and the
kit followed (`bd6cbf775` + `95b45b6cc`): sixteen moves, eighteen of his
twenty-three effects bound, the geyser as his Up-B. He is off `KNOWN_UNARMED`.
⚠ what is left is a BALANCE pass with real eyes — he finished the observed match
at 36% against George's 5% and lost, which is the design's direction but a
bigger margin than intended.

**SECOND HALF — Jon, 2026-08-16:** *"It might be the case that we have to author
more oiler poses so he has a full smash moveset and can use some of his new
sweet sfx and vfx. Especially oil geyser."* He is right that the poses are the
gap, and the census already says so from the other side: `npc_oiler` is one of
the eight fighters on `KNOWN_UNARMED` in `smash_roster_movesets.rs` — a
`peaceful` preset, no melee at all. The rig publishes four rows (idle, walk,
talk, interact), where George Booul's fighting sheet publishes nine and names
them after his MOVES (`toggle_state`, `and_zap`, `not_fade`, `hit`, `death`,
`taunt`). ⇒ the work is: authored moveset row → rig clip → sheet row →
`FxId`/`SfxId` on the move. The effects are already rendered and waiting —
`vfx.oiler.oil_geyser_{emerge,stream,impact}`, `wrench_strike`,
`stabilizer_{spinup,lock}`, `pressure_vent` are in the SFX renderer's output and
`oiler_vfx` is on the sprite roster. ⛔ do not start this before the body swap
above is verified in a capture; a moveset on the wrong body is two problems.

✔✔ **THE SECOND HALF LANDED 2026-08-16** (`3f7d265` in the sprite submodule,
superproject commit below). Sixteen moves in
`game/ambition_content/src/oiler_moveset.rs`, eight new side-view rig clips, and
Oiler's row moved from `peaceful` to `striker_swipe`. **Both census ratchets
moved in the same change and are green**: he left `KNOWN_UNARMED` (8 → 7) and
joined `WITH_REPERTOIRE` (7 → 8) in `smash_roster_movesets.rs`. The Up-B IS the
geyser — a commanded `Set` rise that stages `oil_geyser_emerge` (the tell, before
the burst), `oil_geyser_stream` ×3 across the climb, and `oil_geyser_impact` at
the crest; eighteen of his twenty-three rendered effects are now bound.

⭐ **the architecture carried it with ONE new authoring primitive and no engine
change** — `moveset_authoring::strike_tag`, because `strike` tags every volume
`slash_arc` and a poke wants `slash_poke`. No character-ID branch anywhere.

⛔⛔ **two traps found here, both silent, both now guarded:**
**(1) a `MoveEventKind::Vfx` event plays NO SOUND.** ⚠⚠ **NO LONGER TRUE — D149
fixed it, and this row is kept as history.** It was: the paired
`vfx.<family>.<row>` cue is only looked up on the `FxRequest` path, so a move's
`Vfx` wrote `VfxMessage::Effect` straight through and a perfectly correct effect
name was a perfectly silent animation. The dispatcher asks for the pairing now;
⛔ the remedy this row recommended — authoring every burst as a `Vfx`+`Sfx` PAIR —
is now the DEFECT, because the second half plays the sound twice. See D149, and
`a_paired_burst_is_heard_exactly_once`.
**(2) four oiler cues pack with a `.loop` suffix the sprite row does not carry**
(`vfx.oiler.oil_geyser_stream.loop`, `gate_calibration`, `invariant_loop`,
`portal_leak`), so the mechanically derived cue for those four misses the bank
outright. Nothing strips or adds it. Spelled out at the call site.

⚠ **and the poses were the expensive half for a reason worth keeping.** Oiler's
arms are 26.5px long from a shoulder 25.6px above the hanging wrist — already at
near-full extension at rest. Every hand target picked by eye landed outside the
reachable circle, the IK clamped it, and all eight "big" swings collapsed onto
the same 45° pose. ⇒ **author arm poses as ANGLES on the reachable circle**
(`_arm_envelope` reads the shoulder and bone lengths off the rig document), never
as (x, y) guesses.

**Remaining:** the sheet has one upward and one downward swing, so `tilt_up`/
`smash_up` share `attack_up` and `tilt_down`/`smash_down`/`air_down` share
`attack_down` — honest, and thinner than the table.

- ▢ **D125 — The systemic world substrate: what a thing IS, which occurrence it
  is, why it exists, and how long it lasts.**

⭐ **A THIRD INSTANCE ARRIVED 2026-08-17, from a domain this row had not
touched — and it is the cleanest statement of the row's thesis yet.** D150: a
projectile's ALLEGIANCE was reconstructed every tick by querying the firing
`Entity`, so a shot in flight turned on its own team the moment its firer lost
a last stock and despawned.

```text
what a thing IS            the shot's side          ← was read off a LIVE ENTITY
which occurrence it is     the body that fired it   ← the entity WAS the answer
how long it lasts          the bolt outlives it     ← and that is ordinary
```

⇒ **body RESIDENCY was standing in for stable identity**, which is this row's
whole subject. ⭐⭐ and the same domain had already SOLVED it on the other half:
`inherit_projectile_presentation_sources` says *"the bolt is the emitter … it
routinely outlives the body that fired it. So the source is STAMPED at spawn
rather than looked up at impact."* The presentation half stamped; the COMBAT
half kept counting who was still standing.

⚠ **and D148 was the same error in a third place the same day** — the winner
banner decided "is this side a team" by counting RESIDENT bodies, so a team
whose other member had been eliminated announced its last survivor's name.

⇒ **three independent sites in one campaign, none of them aware of each other,
each fixed by asking a FROZEN record instead of a live query.** That is the
argument for the substrate rather than for three more point fixes — and it is
evidence this row should be worked before the next domain rediscovers it.

✔✔ **THE RESTORE FALSIFIER IS GREEN (2026-08-16, `13dd4d31b`)** — bank a reward
at a checkpoint, carry it to another room, DROP it there, leave so that room
UNLOADS, then die: it comes back **into the hand that banked it**, as the same
occurrence (`SimId` *and* `SpawnOrigin` from the authored record), with its
pedestal still empty and no duplicate. Driven end to end on the composed host
through authored LDtk items, a real `HealShrine` + `Interact`, real door
crossings and a real `ActorDiedMessage`; run red-then-green.

⭐⭐ **the mechanism that was missing was MATERIALIZATION, and the reason is
worth keeping.** The custody restore was pure *re-assignment* — it walked live
objects and asked whether the checkpoint agreed with where each one was, a
question that cannot be asked about an object whose entity no longer exists. No
room build could supply it either, and correctly so: an `InCustody` row makes
`outlook_for` answer `Suppressed` in **every** room, because a thing in a hand is
not a thing in a room. ⇒ **every other reconstruction road in this engine starts
from a ROOM and asks what it owes; this one starts from an occurrence resident in
no room**, so the authored definition has to be reachable BY IDENTITY. No new
rollback state; schema stays v32.

✔✔ **AND THE RUNTIME-MINTED CASE CLOSED TOO** (2026-08-16, `88b611caf`) — the
limitation that stood here is gone. Materialization was bounded by *"some room
authors a record with this id"*; a runtime-minted instance (the throw's
`SimId::spawned` arm, where the inventory count table equips an item with no
object behind the hand) had no record anywhere and was lost on a death. **The
minimal durable description turned out to be three things and no more:**

```text
identity     the occurrence's own SimId
provenance   SpawnOrigin::Dynamic { parent, sequence }
definition   the item spec's authored id — a REFERENCE, never a copy
```

⛔ no position, no velocity, **no component snapshot** — that is rollback wearing
save's clothes. The predicted *"a hand needs strictly less than a world"* held:
`ground_item_physics` refuses to step anything not `InWorld`, so the hand
supplies the place.

⭐⭐ **the coordinator's prediction was one field short, and the missing one is
the durable-save lesson.** I predicted `(identity, spec)`. An instance rebuilt
without its `SpawnOrigin` cannot say which spawner it descends from — the state
that component's doc refuses to let anyone spell — so it would survive **exactly
one death** and then be invisible to the next capture. ⇒ **a durable description
that restores the thing is not sufficient; it must restore the thing's ABILITY TO
BE DESCRIBED AGAIN.** The mint site was not stating provenance at all, so
identity and provenance are now minted as one value.

⭐ **snapshot-not-registry is MEASURED, not asserted**: rebuilt as a growing
registry of every mint with the restore returning each row to its spawner's hand,
the banked-item fixture stayed GREEN and
`a_runtime_mint_the_checkpoint_never_saw_is_not_resurrected_by_a_death` went RED.
`MintedItemBaseline` answers HOW to rebuild; the custody baseline still decides
WHETHER and INTO WHOSE HAND. It lives in the item domain for the same reason the
retraction does — the lifecycle crate cannot see a `GroundItem`'s spec, and what
an item IS is not a question it may answer. Schema 32 → 33.

⛔⛔ **AND IT EXPOSED AN INSTRUMENT DEFECT: A FIXTURE HAD BEEN MEASURING A WORLD
NOBODY CHOSE, FOR ITS WHOLE LIFE.** `with_start_room` takes a ROOM ID;
`central_hub_basement` is an LDtk **level** name. The option warns and falls back
to the authored entry room, so the test silently ran in `central_hub_complex`
while every comment in it named the basement. `StartRoomMustResolve` exists for
exactly this and is **opt-in, and no test in the suite opts in**.
⭐ **blast radius MEASURED 2026-08-16 and it is ~zero**: of all 40 `with_start_room`
call sites, every literal resolves as a real room id **except that one** (76 room
ids vs 77 level names across the shipped worlds; 75 overlap), and the only other
non-resolver is the deliberate `"definitely_not_a_real_room"` negative test. ⇒
making the test harness resolve strictly is now a small, bounded change with one
known opt-out, not the open-ended risk it looked like. ✔ **RAN 2026-08-17.**

⚠⚠ **BUT THERE IS A WRITTEN COUNTER-ARGUMENT IN THE CODE, AND IT NAMES THIS
EXACT HARNESS — read it before running this** (measured 2026-08-17,
`game/ambition_app/src/app/resources.rs:~240`). The strict/tolerant split is
deliberate and reasoned:

> *"a PROGRAMMATIC override comes from a library caller (`Platformer2dSimHarness`,
> the RL harness) that may legitimately name a room outside this composition;
> falling back is the tolerant, correct answer."* — while a CLI flag *"was typed,
> just now, by somebody who wanted that room"*, so it is already strict.

⇒ **so this row proposes reversing a recent, explicit decision**, and doing it
silently would be the thing this campaign keeps catching elsewhere.

⭐ **and the measurement answers the objection: the tolerance has ZERO
beneficiaries today.** Every `with_start_room` literal in the tree is a real room
id —

```text
combat_calibration_lab ×8 · duel_arena ×5 · central_hub_complex ×4 · portal_lab ×3
under_town_pipes · tiny_chamber · symmetry_room · mockingbird_arena
hall_of_characters · goblin_encounter        … and one deliberate negative:
definitely_not_a_real_room ×1
```

⇒ **count the adopters, applied to a BEHAVIOUR rather than a capability**: not
one caller relies on falling back. The written argument protects a hypothetical
future library caller in a composition that lacks the room.

✔✔ **DONE 2026-08-17 (`3f116e88b` seam, `90187a559` migration) — and the shape
was not strict-by-default at all.** A test disagreed in EXECUTABLE form:
`unknown_start_room_does_not_panic_or_error` names, explains and asserts the
fallback, so tolerance is a PROMISE. ⇒ the verb keeps its meaning and the CALLER
states intent — `with_start_room` (tolerant) vs `with_required_start_room`
(refuses to boot, listing all 72 ids). **37 sites across 24 files migrated; one
tolerant literal remains, and it is that promise's own test.**

⭐ **the two highest-leverage sites were NOT in the literal census** — a shared
`fixed_60hz_room_options` helper fanning out to ~20 more fixtures, and the two
`ambition_app_tools` binaries, *"the tool class that produced the original sweep
failure"*. A grep for literals cannot see a helper.

⭐⭐ **and the one real exception changes the COUNT, not the premise: the census
had folded a SENTINEL in with the room names.**
`collision_invariant_oracle::run_episode` takes `start_room: &str` where `""`
means *keep the authored start*. That was never a room id — and it was not
BENEFITING from the fallback, it was ABUSING it to express "no override". Fixed
by passing no start room at all rather than asking tolerantly for one.
⇒ **nothing relies on tolerance as a fallback**; the row's premise held.

⛔ **zero fixtures turned out to be quietly misdirected** — every literal, every
`const ROOM`-style id and every hand-listed array was checked against the live
72-id set. Including `boss_sheet_wiring`, whose `let Ok(..) else { continue }`
would NOT have caught a fallback and was therefore the likeliest silent
passenger. All five arenas real.

✔ **so the shape that honours both is strict-by-default with a NAMED opt-out**
(the negative test takes it, and a future foreign-composition caller has
somewhere to say so) — not a silent flip. ⚠ `StartRoomMustResolve` is opt-in with
exactly TWO adopters today (`capture_scene`, `tests/shield_ring_probe.rs`)
against **24 files** that call `with_start_room`, which is the asymmetry the row
is really about: the RUNTIME is already strict and the HARNESS is not.

✅ **LANDED 2026-08-17 — the seam, then the callers.** `with_start_room` stayed
tolerant and `with_required_start_room` was added beside it (`3f116e88b`), so the
CALLER states which of the two it means rather than the verb changing meaning
underneath both. Then **all 24 files migrated**: every test, helper and tool that
names a room now asks for it strictly — including the two shared helpers
`tests/common::fixed_60hz_room_options` (which fans out to ~20 more fixtures) and
`ambition_app_tools`' `headless` + `rl_smoke` binaries. `cargo test --workspace
--lib` 4867 passed / 0 failed; `app_it` 412 passed / 0 failed.
⭐ **and the defect this item was opened for is gone**: the test that asked for
`central_hub_basement` — an LDtk LEVEL name, never a room id — now names a real
room, and carries a comment saying what it used to say. Every remaining mention
of that string in the tree is prose about the history.
⇒ **both markers above were stale and are flipped**; nothing in this sub-item is
outstanding.

⭐ **the migration was also a TEST of the room ids, and they all held**: not one
call site changed behaviour, so nothing in the tree had been quietly testing
somewhere else. Checked against the live 72-id set — every literal, every `const
ROOM`/`SOURCE_ROOM`/`TWO_ITEM_ROOM`/`ROOM_ID`, and every hand-listed room array
(`boss_sheet_wiring`'s five boss arenas, `content_dormancy`'s four,
`rollback_coverage`'s unswept-population six) resolves. ⇒ **the row's premise
held: nothing relied on the fallback.**

⛔ **TWO deliberate non-adopters, and only one of them was known.** The negative
test `unknown_start_room_does_not_panic_or_error` keeps its tolerant literal by
design — it *is* the promise. The one the measurement had missed:
**`collision_invariant_oracle::run_episode` uses `""` as its OWN sentinel for
"keep the LDtk-authored start"** (`collision_oracle_smoke` passes it), so it was
never a room id at all and never a beneficiary of the fallback — the count of 40
literals had folded a sentinel in with the room names. It now passes **no start
room** rather than asking tolerantly for one, which is the honest spelling:
`with_required_start_room("")` would rightly refuse to boot, and asking
tolerantly for `""` is the silent substitution this row exists to end.

⚠ **and a stale comment that the fallback had authored got corrected**:
`rl_smoke` explained a soft-fail branch as "start_room override fell back", which
is now unreachable — an id from `room_ids()` asked for strictly refuses to boot
instead. Reaching that branch now means something else entirely (an active area
under another LDtk name, or an immediate transition), and it says so.

⭐ **PROMOTED FROM THE RESERVOIR 2026-08-14, and the promotion IS the work that
was missing.** Measured: seven focused plans for this frontier already exist and
are already good —
[`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md),
[`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md),
[`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md),
[`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md),
[`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md),
[`engine/persistent-actors-and-population.md`](engine/persistent-actors-and-population.md),
[`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md).
**All seven were reachable only from [`tracks.md`](tracks.md), and none from this
queue** — so the frontier was fully designed and structurally unreachable from
the execution authority. Nobody needed to write a brief; somebody needed to
replenish the ledger, which is what `tracks.md` exists for.

**Sequence, and it is a sequence — each step's identities are what make the next
one expressible:**

1. ✔ **The substrate is ALREADY BUILT, under names this plan does not use —
   measured 2026-08-14, and the census overturned every expectation the brief
   carried.** ⛔ **do not build it.**
   - *What authored thing is this?* → `WornCharacter(CharacterId)` on the body,
     resolved through `PreparedCharacterRegistry`. Its doc already splits NAMING
     a template from APPLYING it (`RecharacterizeBody`), so it already refuses
     the identity-implies-uniqueness conflation.
   - *Which runtime occurrence is this?* → **`SimId`**: deterministic,
     namespaced (`placement:` / `slot:` / `encounter:` / `spawned` / `strike`),
     `#[require(SimIdCounter)]`, dynamic spawns minted as `(spawner SimId,
     per-spawner counter)`. Every snapshot row, checksum projection and
     cross-reference keys on it, and the module opens by explaining why an entity
     index cannot be an identity. ⭐ **the prediction was "this exists only as
     `Entity`" — it is the opposite.**
   - *Why does it exist?* → **`SpawnOrigin`**, a component:
     `Authored{source,instance}` / `ProviderStaged{provider,room,instance}` /
     `Dynamic{parent: SimId, sequence}`, `parent` non-optional by design, verified
     against the construction plan roster and encoded into rollback blobs. Its
     module states the rule outright: *provenance is data, never recovered by
     parsing an id string.* ⭐ **provenance was predicted to be the big absence and
     is the best-built part of the four.**
   - *How long should it last?* → four ENFORCED scopes, each owning a sweep:
     `RoomScopedEntity`, `ModeScopedEntity`, `RoundScopedEntity`,
     `SessionScopedEntity`, plus per-domain TTLs and `EncounterCleanupPolicy`.
     `round.rs` already states this model's hardest rule unprompted: *"round scope
     is a LIFETIME, not a provenance — where an entity CAME FROM does not say how
     long it should live."*

   ⛔⛔ **the one real gap was a FALSE DECLARATION, not an absence, and it is now
   deleted.** `RunScopedEntity` and `PersistentEntity` — with `spawn_run_scoped`
   and `spawn_persistent` — had **zero producers and zero consumers**, and no
   sweep read either marker. That is worse than a missing lifetime, because
   `lifecycle/mod.rs` directed new spawn sites to `SpawnScopedExt` and **two of
   its four verbs silently did nothing**: a call site writing `spawn_run_scoped`
   declared "dies with the run" and got an entity outliving every boundary the
   engine has. `RunScopedEntity` was the unenforced half of a FORK —
   "dies with the run, survives room transitions" is exactly `SessionScopedEntity`,
   which is id-carrying and actually swept. `PersistentEntity` was a second
   spelling of ABSENCE: every sweep culls on marker *presence*, so an unmarked
   entity already survives all four boundaries, and the marker invited the false
   belief that it OVERRIDES a scope — which it cannot, since
   `(RoomScopedEntity, PersistentEntity)` still despawns on room unload.
   ⇒ the surviving rule is now written down: **a scope is spelled here only if a
   sweep enforces it.**

   ⇒ **what genuinely remains in step 1** is small and both halves are listed as
   unresolved by the focused plan itself: **persistence policy** and **the
   explicit terminal transition**. ⛔⛔ **and one distinction must not be lost:
   "has no runtime cleanup scope" does NOT mean "durably persistent open-world
   object that is correctly saved and restored"** — an unmarked entity merely
   survives this session's four boundaries. The durable-persistence system is
   still undesigned, and reading the deletion of the fake markers as "persistence
   is handled" would be the exact error those markers caused in the first place. ⚠ plus one concrete player-centrism smell in
   `room_transition/commit.rs`: whether a transiting body survives the room change
   is inferred from `!self.bodies.presentation.contains(subject)` — *"does it
   carry home-only presentation state"* — which is either a no-op or catastrophic
   (if that proxy ever fails, the player is despawned mid-transition). Collapsing
   it to `Some(subject)` is strictly safer; the `presentation` query itself must
   stay, since the blink camera also uses it.

   ⛔ authored identity does NOT imply world uniqueness — "there is normally one
   Fia" is content policy, not the meaning of a definition id. ⛔ **do not invent
   a universal `EntityId`**; the separation is settled, the exact identity types
   deliberately are not.
2. ◐ **Item custody — and BOTH of my briefed findings were wrong, which is the
   valuable part (2026-08-14).**

   ⛔ **"item entities carry no `SimId`" is FALSE on the authored path.**
   `construction::authored_ground_item_requests` builds every LDtk `GroundItemSpec`
   row with `SimId::placement(&spec.id)` + `SpawnOrigin::Authored`, and the
   construction executor stamps both onto the allocated root before the recipe
   runs. `ensure_sim_id`'s `With<BodyKinematics>` filter was a red herring — items
   never went through it and never needed to. ⭐ **the real defect was adjacent and
   worse: the identity that DID exist was DESTROYED at the first custody change.**
   Pickup called `despawn()`, throw called an unconditional `spawn_room_scoped`.
   The gap was never missing identity; it was destroy-and-recreate discarding it.

   ⛔⛔ **and the pickup fork is SYMMETRIC — neither population was correct.**
   `collect_ecs_pickups` claimed `With<PlayerEntity>`: N couch seats, excluding a
   possessed body, which is the reported bug. But `collect_world_items` read
   `Res<ControlledSubject>` and served **exactly ONE body**, so **seat two picked
   up coins and not mushrooms** — a half nobody had reported. ⭐ **unifying onto
   `ControlledSubject`, the obvious fix, would have cut couch collectors from N to
   1**, which is exactly the reduction hazard an earlier agent refused to walk
   into. Now a filter-plus-value population serves both, pinned by a test that
   asserts a second seat AND a possessed body collect — **paired deliberately,
   because either half alone passes on the broken code.**

   ⇒ **the instance / quantity / consumable split is now written on `ItemCustody`:**
   a `GroundItem` is an INSTANCE and keeps its identity across world → held →
   world; `PickupKind::Currency`/`Health` and the `OwnedItems` counts are
   QUANTITIES (⭐ *two coins are the same coin — what survives is a number on the
   collector*); a `WorldItem` is a CONSUMABLE whose despawn is a real end of life.
   One `spawn` survives for the case that genuinely mints: a body equipped from the
   count table has no object behind its hand, so throwing turns a quantity into an
   instance — and that instance takes `SimId::spawned(thrower, counter.next())`
   rather than joining the world anonymously.

   ⚠ **`ItemCustody` IS rollback state and is registered as such** (`item.item_custody`,
   clone + entity-SET probe since `InWorld` names no body, paired with
   `rollback_map_entities`); schema **29 → 30**, both baselines updated. It gates
   drawing, physics and grabbability on later frames and took over a job GGRS
   previously did through the entity anchor.

   ⛔ **the INVENTORY leg is explicitly NOT closed, and says so in code.**
   `OwnedItems` is a global count table with no row per object — which is precisely
   why *whose inventory does a possessed body fill?* still has no answer, and why
   `equip_held_spec` / `unequip_held` are labelled a **migration seam** rather than
   the model. ⭐ physical custody belongs to the body and the item instance;
   participant entitlement is a separate fact with a different owner and lifetime.

   ⇒ **carried forward:** dynamic drop sites still mint no identity
   (`damage/boss_hit.rs`, `damage/actor_hit.rs` spawn `GroundItem` drops with no
   `SimId`, so a boss-dropped axe has nothing to preserve); an orphaned custody is
   possible if a holder despawns while carrying (inert, bounded by room/session
   scope, no reaper built because that would be machinery for a state nobody has
   demonstrated); and `ChestFeature::reward()` still has **zero production
   callers** — authored chest rewards are parsed, lowered onto the live component
   and never granted.

3. ◐ **Capability-driven gating and platformer reachability — FIRST SLICE LANDED
   2026-08-14, and it avoided the trap by DRIVING THE REAL KERNEL.**

   ⭐ **the design the agent arrived with was dissolved by its own census, and that
   is the win.** It intended a closed-form reachable envelope with a hand-enumerated
   capability list — **which is the deleted "airborne + below the lip ⇒ already
   dead" rule in new clothes**, and would have failed the same way. Three findings
   killed it: `BodyClusterScratch` is already *"a whole body without an entity"*;
   `ae::step_motion` is **pure** (no Bevy `World`); and `movement/containment.rs` is
   the worked example of driving the real kernel N steps against a `&ae::World`.
   ⇒ **`movement/recovery.rs` clones the body and drives ITS OWN kernel** over three
   fixed ordered efforts, reporting `Regained { steps, side }` or
   `NoSupportFound { reset }`.

   ⭐⭐ **why it is not the deleted rule: IT STATES NO RULE ABOUT BODIES.** Every
   capability the kernel implements is honoured because the kernel honours it, gated
   by the body's own `AbilitySet` and `AxisSweptParams` — **there is no capability
   list here to fall out of date, which was the deleted rule's entire failure
   mode.** ⚠ the effort is a *reactive rule*, not a search: hold the side, hold
   jump, re-press the instant the body stops rising — because pressing every tick
   burns a whole air-jump budget in consecutive frames, while pressing at apex
   chains the most height and holding between presses stops a variable-jump law
   cutting the arc.

   ⇒ `recovery_capability_gap(..) -> Option<AbilityGrant>` answers *"which
   capability blocks the route"* in the engine's existing authoring vocabulary, and
   skips a grant that adds nothing — ⚠ **re-granting `AirJump` to a body that HAS
   the verb and SPENT the charge would refill the budget and report the verb as
   missing when the charge was.**

   ⛔ **the consequence is open: NOTHING reads either function.** No call site
   outside the module's tests, nothing wired into the fighter's search, no fighter
   tuned. `NoSupportFound` carries `reset: Some(cause)` only when EVERY effort ended
   in a world reset — *"the world killed it whichever way it steered"* versus
   *"still falling when I stopped watching"* — and the module doc says a brain, a
   validator or an LLM decides what that means. Nothing added is rollback state.

   ⭐ **the pins are three, each with its falsifier inside it**: the body's own kit
   decides (same world, same position, same velocity, only `move_horizontal`
   differs, and BOTH terms asserted so ignoring capabilities fails); the deleted
   rule's exact `doomed` state is answered by SURFACES (poison: remove the one catch
   block and the identical body must report not-recovered); and the probe is
   **gravity-generic** (room transposed, gravity along `+x`, with the non-steering
   body still failing so it cannot pass by reporting success for everything).

   ⇒ **a DELETION GATE was named rather than taken**, on the fighter rollout's
   duplicate integrator — whose own doc already concludes *"the fix is not different
   constants, it is DERIVATION."* Delete it when three things hold: the brain can
   obtain a `&ae::World` without depending on `ambition_platformer2d_world`; one
   real kernel step per shadow step is measured affordable against
   `rollout_k × (1 + rollout_depth)`; and `ladder_rig --scenarios` re-runs green —
   **the only instrument that has ever caught a shadow-physics divergence.**

4. **Capability-driven gating — the GATING half.** The robot navigates
   because of body capabilities, equipment, physical properties and changed
   mechanisms — **never protagonist identity or a quest flag**. Reachability
   should answer engine/agent questions: can THIS body reach there, which
   capability blocks it, how do portals / moving platforms / gates change the
   route.
4. **Open-world residency and persistent populations, last.** World existence,
   room residency, full simulation and visibility become distinct; named
   actors/items/world changes survive room absence; spawned populations get an
   explicit lifetime policy. ⛔ background simulation stays DELIBERATELY
   UNRESOLVED until evidence says how much is needed. Different-room multiplayer
   then falls out of the model instead of becoming a special multiplayer world.

⇒ **prerequisites, both nearly met:** D71/D92's converged room-transition
transaction (done) and D116's view separation (in flight). Residency is the step
that needs them; the substrate itself does not, so step 1 may start now.

⛔ **NOT IN THIS ROW:** a quest/story framework, a dialogue engine, a generic
scripting layer, production networking, or substantial authored story. The target
is a large coherent 2D world the robot can traverse, alter, leave, return to,
save and reload — **authored story comes after that world exists**, not to
motivate it.

⭐⭐ **THE CUSTODY SEAM IS MEASURED (2026-08-15) AND IT IS ONE CLASS WIDE.** Full
partition and evidence: [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md).
In short: instance-capability is decided solely by whether `Item::held_item_id()`
resolves; **9 held weapons are both an instance and a count, and the other 5
classes are counts forever** whose readers legitimately want a quantity. ⛔ so do
not give the count table a row per object.

⚠ **the flattening is `items/pickup/mod.rs:616`**, and it composes into an
unbounded duplication loop (equip from the count with no object behind it → throw
→ the mint arm materialises a second axe). ⛔⛔ **not fixed, deliberately**:
`owned.take` on throw destroys thrown items on room exit, because **the count is
currently the durable-save mirror of an instance**, not entitlement.

✔✔ **THAT PROOF LANDED 2026-08-15 — a carried item now crosses a room boundary,
and the transition still knows nothing about items.** ⭐ **the mechanism is a
projection, not an entourage**: `ItemCustody` is projected onto an `InCustodyOf`
marker, and the roster a room CHANGE retires became
`RoomResident = (With<RoomScopedEntity>, Without<InCustodyOf>)`. The item **never
loses its room scope** — so a *reset* still destroys it, correctly — and back
`InWorld` the marker goes away, making it resident in whatever room is active
then. ⭐ **room residency carries no room id, so "dropped in the destination"
needs no memory.**

⭐ **it is body-generic for free:** a holder that is itself a room fixture (an
unpossessed NPC) leaves the item resident, so it dies with the room like its
holder; a despawned holder makes it resident again, so the death-drop orphan no
longer escapes every sweep; and possession already promotes a taken-over body out
of room scope, so a possessed carrier gets the travelling answer **without anyone
asking who the player is**.

✔ **deleted in the same slice:** `world/rooms/load.rs` —
`commit_room_transition_geometry` + `RoomLoadResult`, a public 60-line **second
copy of the room commit with zero callers**, carrying its own
`With<RoomScopedEntity>` roster, i.e. a second place this rule would have had to
be applied.

⭐ **I ran the poison the worker could not:** reverting the roster to plain
`With<RoomScopedEntity>` fails the carried-item test exactly, and leaves the
reset test green — the right split. `derived.custody_residency` is a
**declared-derived** row, not a registration (recomputed unconditionally each
tick, no "already applied" gate), so **schema stays 31**. Verified: app_it 363,
Smash 18, monolith 1230, contracts 27/27.

✔✔ **CONTINUITY'S FIRST LEG LANDED 2026-08-15 — a rebuilt room now asks what
became of the occurrence it minted last time.** ⭐ **and the question it asks is a
DISPOSITION, never *"is something with this id alive"*:**
`OccurrenceDisposition::{Authored (default), Persisting, Consumed}`. Construction
gained a sixth stated authority (`occurrences: Option<&AuthoredOccurrences>`,
under the same *"state it, including stating `None`"* contract the cast and brain
policies use, so a seventh road cannot silently forget), and
`RoomFeatureConstructionPlan::prepare` retains only requests whose disposition
`authors_a_fresh_occurrence()`.

⭐ **`Consumed` is spelled and read but has NO PRODUCER — deliberately.** It is the
honest slot for permanent destruction, and it makes *"ephemeral / resettable"* the
**default** rather than a special case. ⇒ that is the shape the terminal cases
need, reserved before they exist rather than retrofitted.

✔ **deleted:** `RoomConstructionPlan::prepare(&World, ..)` — 84 lines, **zero
callers**, and the eighth road an added authority would have been forgotten on;
`RoomConstructionError::MissingService` went with it as its only raiser. Also
tightened: the prefetch cache now states the dispositions it froze and refuses a
plan prepared against different ones, and `ConstructionPlan::prepare`'s `live`
argument — previously `&Default::default()` under the sentence *"nothing it
constructs is live yet by definition"*, **false since the custody slice** — now
carries the suppressed set, so a future road reaching a suppressed identity gets
`IdentityAlreadyLive` at preflight instead of a duplicate.

✔✔ **AND ITS OWN NEW TEST FOUND TWO SEPARATE FAULTS — one instrument, one real
defect. Both closed 2026-08-15.**

⭐⭐ **THE REAL ONE: a sandbox reset rebuilt the room and then emptied it.**
`process_new_game_reset_request` retires every `RoomScopedEntity` and commits the
fresh room plan in one call — and it is `.chain()`ed with
`clear_transient_on_sandbox_reset`, whose `Or<(.., With<GroundItem>, ..)>` query
then despawned the items the reset had **just authored**. ⛔ **a plain `.chain()`
carries an auto-inserted `ApplyDeferred`, so the later sweep SEES what the
earlier one spawned** — "these run in order" is the mechanism, not a defence.
Authored ground items are room-scoped (`insert_room_in_session`), which is what
put them in both queries.

✔ **the fix DELETED THE OVERLAP** rather than reordering: the transient sweep
gained `Without<RoomScopedEntity>`, because `retire_outgoing` already sweeps room
scope unconditionally and is the stricter of the two. ⭐ it silently fixed the
same latent defect for an authored `PortalGunPickup`. ⭐ **verified here, and the
poison too**: removing the filter reddens both the unit guard and the integration
test; restoring it turns them green.

⇒ **what a sandbox reset actually restores, stated once so nobody has to
re-derive it:** the start room from its authored records **alone**, stating no
dispositions — every authored placement, feature and actor comes back exactly
once, *including one that was being carried*, because the reset destroys the
world those occurrences live in, hands included. ⛔ **it restores nothing outside
room scope**: a dropped weapon, a summoned ally, a placed portal and the player's
held state are retired by the transient sweep and never come back.

⚠ **the instrument fault, and it is worth reading twice.** The test *"a carried
object survives a reset"* was **never running the reset it described**.
`reset_episode()` presses `ControlFrame::reset_pressed`, which the host turns into
`reset_sandbox` + `ResetRoomFeaturesEvent{Manual}` — a road that restores room
FEATURE state in place and **never touches `RoomScopedEntity`, never empties a
hand, never re-runs authored construction**. The reset the assertions describe is
`process_new_game_reset_request`, reachable only through `NewGameResetRequested`,
whose writers are the kaleidoscope menu and tests — **nothing on the input path**.

⛔⛔ **and the failure was HALF SILENT, which is the lesson.** The sibling's
`occurrences(..).len() == 1` passed **vacuously**: the surviving carried item WAS
the one occurrence, so **a reset that did nothing at all satisfied it.** ⇒ two
assertions over the same state can agree with each other and with the bug.

✔ the behaviour was correct all along — the item keeps `RoomScopedEntity`, has no
`PhysicsRoomEntity`, and the real sweep despawns it. The test now drives the real
reset with **every original assertion kept verbatim**, plus one the count cannot
see: the **carrier's hand is empty afterwards**. The sibling's prose no longer
infers a sweep it never exercises, and `reset_episode`'s own doc now says what it
is **not**.

⛔ **the deletion gate, refused on purpose:** collapsing the two reset roads is the
only deletion available, and `session/reset/mod.rs:165` already argues one sweep
cannot answer both *"does this survive leaving the room"* and *"does this survive
replaying it"*.

⚠ **a real latent gap found and deliberately NOT fixed:**
`clear_transient_on_sandbox_reset` scopes hand-emptying to `With<PlayerEntity>`,
so **a non-player carrier keeps a dangling `HeldItem`** after the reset destroys
the object. ⛔ the simulation is not player-centric and this road still is. It was
left alone because the same loop also restores `ActionSet`/`StashedActionSet` and
strips `PortalGun`, which are genuinely player concerns — ⇒ **relaxing the filter
is a product call, not a refactor.**

⇒ **remaining legs of the same question:** the two terminal cases —
**destroyed permanently** ⇒ never recreate (needs a `Consumed` producer) and
**intentionally resettable** ⇒ may recreate (already the default). Plus two
carried risks the author recorded: `SimId::placement(id)` is a **global**
namespace whose uniqueness is only checked **per room**, so two rooms authoring
one id would suppress both; and the ledger is not experience-scoped, so a
suppressed row can survive into a new session.

⇒ the hazard that started this leg, recorded on `ItemCustody`:
carrying an **authored** placement out of its room and back yields the carried
object **and a freshly authored copy with the same `SimId::placement(..)`**. It
could not arise while the boundary destroyed the object.

⭐ **but the hazard is the small statement of a much bigger question**, and this
is the systemic-world pressure the project has been trying to reach:

> when authored placement **P** has produced a runtime occurrence that has since
> **moved**, been **consumed**, been **destroyed**, or entered **custody
> elsewhere**, how does world reconstruction know what should happen to P?

It sits underneath persistent items, moved NPCs, opened/removed mechanisms,
destroyed objects, relocated quest objects, persistent populations, room
streaming and save/load. Design owner:
[`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md),
where it forced two questions off the *deliberately unresolved* list.

**Falsifier:** `enter A → axe P exists → pick up P → carry to B → return to A →
P must NOT respawn, and the original occurrence still exists elsewhere.`
Terminal cases to follow: **destroyed permanently** ⇒ never recreate;
**intentionally resettable** ⇒ may recreate.

✔✔ **THE CHECKPOINT/RESET HORIZON LANDED 2026-08-15.** Seven beats of the
maintainer's rule hold end to end through production roads
(`death_restores_the_checkpoint.rs`, `central_hub_basement`): an object acquired
before any checkpoint goes back on its pedestal; acquired-then-banked stays in
hand with the pedestal empty; and one death reaches **two opposite answers about
two objects of the same kind in the same frame**, separated only by which side of
the checkpoint each acquisition fell on. ⛔ that last beat is what no
`KeyItem => survives` rule can produce.

⭐ **the baseline is a projection of DOMAINS, not a resource.**
`lifecycle::horizon` owns two messages and two sets and nothing else;
`OccurrenceBaseline` and `CustodyBaseline` are captured by their own domains from
their own live authorities. Both are checksummed rollback state (schema v32) —
the first values here that nothing republishes, so the derived declaration the
live ledger enjoys would be a lie for them.

⭐⭐ **three defects the fixture found that reasoning did not**, and all three
were found by RUNNING it rather than by poisoning it:
1. **restoring the ledger and emptying the hand DELETES the object** — the room
   replay resets features in place and never re-runs authored construction, so
   nothing authored it back. ⇒ a death is a checkpoint RESUME: it records the
   same `LifecycleIntent::Transition` a session-start resume records, and
   same-room re-entry rebuilds correctly.
2. **custody is a FORKED relation** — `ItemCustody` on the object,
   `HeldItem` on the body. Retracting one half left the body holding a ghost and
   refusing every future pickup. ⛔ the tempting generic repair ("empty a hand
   matching nothing in custody") would disarm every authored fighter, because a
   character definition's `held_item` needs no world object.
3. **a hand must be EMPTIED before it can be FILLED** — interleaved, the
   reinstatement was equipped over an occupied hand and `return_released_items`
   quietly undid it one phase later.

⭐ **the gap that looked save-destroying is NOT** — measured, not reasoned
(`a_banked_object_left_in_an_unloaded_room_survives_a_death`). A baseline row
whose occurrence has no live entity (banked, carried next door, put down, that
room unloaded, then a death) would seem to be erased: the restore overwrites the
`Placed` row that was the only memory of where it lay, after which every room
suppresses it. ⭐ **`republish_custody`'s retract-by-RESETTING rule saves it in a
case it was not written for** — the custody leg is rebuilt from live state every
tick, so the unsupported `InCustody` row is dropped and the home room authors the
object at its pedestal. ⇒ the player loses the *acquired* property they banked,
which is wrong but recoverable; the object is not destroyed, which would not be.

⚠ **and that safety is CONDITIONAL, so it is pinned by a characterisation test.**
It holds only because nothing lets an `InCustody` row outlive live custody. ⛔ a
durable save that writes the ledger straight to disk breaks exactly that, and the
annihilation becomes real.

⭐⭐ **MAINTAINER DECISION 2026-08-15 — the CHECKPOINT is the reset baseline.**
Death/retry restores the latest committed checkpoint; traversal and unload
preserve current state. ⛔⛔ **not** `KeyItem => survives reset` — a key item
persists because acquiring it **committed a checkpoint**. ⇒ the owner must
distinguish **three** horizons: current occurrence state · state at the
reset/checkpoint horizon · durable save. Fixture and full text:
[`maintainer-decisions.md`](maintainer-decisions.md) and
[`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md).

⛔⛔ **do not answer it by teaching the room loader to inspect inventories** —
that is another composition census, and the landed slice's whole achievement is
that room transition never learned items exist. ⛔ and no universal instance
registry: the abstraction belongs around the **disposition of the authored
occurrence**, with storage discovered from this customer.

✔ **inventory OWNERSHIP is settled and is NOT this work:** the **body** owns its
inventory and capabilities; participant entitlement and possession transfer are
separate concerns. `OwnedItems` is a migration representation. ⛔ do not start
that migration ahead of this — placement continuity has the stronger pressure.

✔ **SETTLED — that unknown is closed. The BODY owns its inventory and
capabilities.** Participant entitlements and possession-transfer policy are
separate concerns with different owners and lifetimes, so `OwnedItems` is a
**migration/compatibility representation**, not an undecided authority. ⛔ do not
re-open it, and ⛔ do not start the `OwnedItems` migration ahead of persistent
occurrence continuity, which has the stronger product pressure.

✔ **landed meanwhile, needing no product decision:** stow and equip-swap left an
object recording `ItemCustody::Held` by a body with an empty hand — a third state
the enum does not have — so **an authored axe silently ceased to exist through the
menu**. Custody is now re-derived from the hand and **RESET** to `InWorld`.
⛔⛔ **its test pins the FUNCTION, not the WIRING**: removing the system from the
production chain leaves it green, because the test lists the system in its own
chain. Recorded, not answered with a registration assertion.

⛔ **and Bevy-crate extraction is a CRITERION APPLIED AT EVERY STEP, never a
follow-up cleanup campaign.** Reach a coherent internal `Plugin` with owned
components/resources/messages/system sets and no upward registration; extract to
a workspace crate when dependency isolation is genuinely real; call it
independently consumable only after a small external-style `App` uses it with no
Ambition content or policy. ⭐ **never carve because a file or crate is large** —
that is the failure mode this repository has already named twice.

- ▢ **D72 — Continue Smash as a body-generic combat customer.**

Use [`smash-body-generic-combat-2026-08-09.md`](smash-body-generic-combat-2026-08-09.md)
and [`demos/super-smash-siblings.md`](demos/super-smash-siblings.md). The old
migration diary is archived; only source-confirmed residuals should generate
work.

Super Smash Siblings may eventually become a first-class game, but **Ambition
remains the flagship**. Keep both on shared body/combat/participant/world
semantics rather than adding Smash-only engine paths.

⭐⭐ **RECONCILED 2026-08-17 — and the headline is that this row's OWN RULE was
vindicated seven times in one campaign.** The rule is *"keep both on shared
body/combat/participant/world semantics rather than adding Smash-only engine
paths"*, and every defect the smash work surfaced turned out to live on the
SHARED floor, not in smash:

```text
D155  every authored launch_dir in the GAME was vertically inverted, and a
      tumbling launch resolved as a LANDING — hit_response + the movement
      kernel, both Ambition's too
D114  hitlag was read by the AVATAR road only, so any hit between two ACTORS
      froze neither — Ambition's enemies had it as much as smash's CPUs
D157  the ability gate that should stop a body attacking NEVER EXISTED:
      `combat_actions` derived its slots from the moveset and the ActionSet,
      which say what the attack IS, never whether the body may attack
D150  a shot's allegiance was reconstructed from the firing ENTITY, so it
      turned on its own team when the firer despawned
D156  a sheet's drawn facing was authored three times and read zero times —
      and the BOSS renderer had had the missing XOR since the mockingbird
D154  an authored effect's position was body-local and its ARTWORK was not
D152  empowerment expiry was every game's to remember, and five did
```

⇒ **not one of these was a smash-only path**, and not one was fixed by adding
one. ⭐ what smash did was PRESSURE: it is the first customer that seats
fourteen bodies, runs them CPU-vs-CPU, and launches them — so it reads floors
that Ambition's own play had never leaned on. That is the argument for keeping
the row's rule rather than the argument for a second engine.

⚠ **the residuals this campaign leaves are named and small**: D158's
speech-bubble stacking (closed 2026-08-17 — the offsets were separated and the
LINES were not), and one thing that turned out NOT to be a residual at all on
inspection.

⛔ **I filed "presentation hitstop is slot-0 only" as a defect and it is a
DESIGN FORK — corrected the same day, before anyone spent a session on it.**
Reading `emit_player_time_intent_system`
(`actor_monolith/src/time/time_control/mod.rs:307`) says three things:
1. **D114 already fixed the part that matters.** Both movement roads spend
   hitlag now, so BOTH BODIES STOP on a connect — that IS the visible impact.
   The clock request only adds freezing everything else on the sim clock
   (particles, VFX, other bodies) as a flourish on top.
2. **slot-0 is CORRECT for what this system is for.** Its other arms are
   bullet-time and blink-hold — per-PLAYER feel affordances by ADR 0010/0011.
   Slot 0's blink slows slot 0's world; a second player would emit its own
   intent against its own clock.
3. **in a CPU-vs-CPU match there is no `PrimaryPlayer` at all** — and this file
   already carries Jon's 2026-08-07 freeze from exactly that shape (a paused
   match forced the clock to zero, and with nobody to ask for the neutral pace
   back the world ran at scale 0.0 forever, *"the characters are just stuck in
   air"*).
⇒ **so "whose hitstop owns the SCREEN when nobody is playing" is a real question
with several defensible answers** — nobody's, the most recent hit's, the framed
fighter's — and not a bug with an obvious fix. ▢ recorded as a fork; do not
guess it.

✔✔ **THE SELF-KO CAUSE IS FIXED (2026-08-15), and it was ARCHITECTURE, not
tuning.** The measured defect — depth 12 survives 7.4s while depth 0 survives
47.8s, a duelist losing three stocks to itself at 0% — is answered by giving
`probe_recovery` its first consumer rather than by another shortcut.
`RecoveryLens` lowers the perceived view into a real `ae::World` and the body's
**own** `AbilitySet`/`MovementTuning` into a scratch body, and
`refine_by_rollout` now lets the kernel overrule the shadow **both ways**.

⭐ **the REPRIEVE, not the condemnation, was the missing half** — and nobody
predicted that. The shadow line `Hold`s after `commit_ticks`, so near a ledge it
condemned **every** verb; the veto emptied and choice fell through to
`least_bad_movement`, which picks *dies latest*, not *lives*.

✔ **deleted in the same slice:** `WorldView::reachable` and
`SolidKind::blocks_path` — hand-rolled straight-line reachability with **zero
production consumers**, i.e. exactly the duplication the reachability plan
forbids, sitting in the tree while that plan was being written.

⭐ **I ran the falsifier and it holds:** two bodies at an identical position with
identical geometry, gravity and unspent air-jump count, differing only in
`AbilitySet::double_jump`, reach opposite verdicts. Poisoning the production path
so the lens uses a default `AbilitySet` reddens **exactly that test and no
other** — so the verdict comes from the body's capabilities, not the stage's
shape. ⛔ that is the property the deleted *"airborne + below the lip + outside
the span ⇒ dead"* shortcut could never have.

✔ **THE LEDGE TRANSITION IS CLOSED (2026-08-15), and it was one predicate.**
The shadow's walk-off test compared the body's **centre** to `ground_span` while
`surface_supports_body_at_rest` compares its **FOOTPRINT**, so `left_the_ground`
captured a position a half-extent early — 11px on the shipped fighter — at which
the real kernel still found the body standing, and reprieved the walk-off with
the very platform it was leaving. The fix is not a nudge: `integrate` now calls
`ae::collision_semantics::spans_overlap_for_support`, the 1-D core extracted out
of `perpendicular_overlap`, so the shadow asks the kernel's own question with the
kernel's own `EDGE_OVERLAP_SLOP`, and the centre test is DELETED.
`Perceived::supporting_floor` carries the identical correction from 2026-07-31;
this was the last centre-in-span ground test in the fighter's ledge reasoning.

⚠ **two limits still recorded, not hidden:** blast margins are probed at **zero**
while the stage authors 120px (conservative by exactly that; it does NOT interact
with the ledge case — the trivial reprieve came off a *platform*, not off the
envelope) · cost is unmeasured, and the existing budget bench rolls no movement
lines so it does not price the lens. Probe FREQUENCY is unchanged by the ledge
fix — still ≤1 per modelled verb per decision, from the same lines; the capture
point simply moves ~2 ticks later along each of them.

⚠ **SUPERSEDED 2026-08-15: the pause is lifted, replaced by D128.** The old note
read *"do not tune the fighter further until higher-leverage architecture work is
exhausted"* — the maintainer has since opened the combat-expression lane
deliberately, as a product-pressure slice. ⛔ **its licence is still not tuning**:
D128 authors repertoire and adds reusable engine primitives; a dial-turning pass
remains out of scope. The deterministic evaluation rig
and its measurements stay. The S4 diagnosis is recorded in
[`engine/fighter-brain.md`](engine/fighter-brain.md): the rollout horizon is ~12
ticks / 0.2s, the fall from platform to blast floor is ~24 ticks / 0.4s, so a
deeper search cannot see the cost of a ledge exit and increasingly picks
apparently-free self-KO trajectories. ⛔ the "airborne + below the lip + outside
the span ⇒ already dead" terminal value was implemented, measured, and REMOVED:
it is not body-generic (air movement, jumps, flight, wall interaction, ledge
grab, recovery attacks, impulses, portals, grapples all falsify it). A real
committed-fall value comes from recoverability under the body's own capabilities
— eventually a consumer of
[`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md)
— or from a horizon long enough to contain the landing.

- ▢ **D33 — Continue actor-monolith decomposition by coherent ownership.**

Use [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).
Choose carves from current dependency and authority measurements, not an old LOC
target.

⛔⛔ **STEP 3 WAS ATTEMPTED AS THE `encounter` CARVE ON 2026-08-17 AND IS
REFUSED — BOTH PRECONDITIONS BELOW ARE FALSE, AND THE MEASUREMENT THAT PRODUCED
THEM WAS TAKEN WITH THE WRONG INSTRUMENT.** The block below is the original
proposal; it is kept because the correction only makes sense against it.


⭐⭐ **RESOLVED 2026-08-17 — the monolith is UNDER its frozen baseline for the
first time, and `largest_unit_lines` has left the findings list entirely.**

```text
111,429   the frozen baseline (2026-08-09)
121,822   2026-08-17 morning        +10,393 over, ~5× budget
114,139   boss_encounter CARVED     725de8c26   −7,683
110,932   four modules RELOCATED    355874fe1   −3,207   ⭐ UNDER baseline
```

⭐ **the second slice created NO new crate** and cost no hop
(`critical_path_crates` 13 → 13 → 13, where the `conversation` carve had cost
12 → 13). It moved modules whose owning crate ALREADY existed and deleted a dead
1,336-line `persistence` module outright.

⚠ **the growth finding below is still true about the eight days it measured** —
kept because the lesson stands: this row cannot be judged by "did a crate leave
this session", and broad feature growth can outrun one carve per session. What
changed is that two slices in one day beat it.

⛔⛔ **THE ROW'S SCOREBOARD SAID DECOMPOSITION WAS LOSING GROUND — and it was,
for eight days. ⭐ RESOLVED THE SAME DAY; the arc is below.** The
compile ratchet's baseline was frozen 2026-08-09. Since then:

```text
largest_unit_lines  ambition_platformer2d_actor_monolith
                    111,429 → 121,822   (+10,393, budget was +2,228)
```

⭐ **confirmed by a second, independent route** before reporting: counting `.rs`
lines from `git ls-tree` at the freeze commit vs `HEAD` gives 112,201 → 122,599,
**+10,398**. Two instruments, one conclusion — the monolith gained ten thousand
lines in eight days while this row carved one module out of it.

⚠ **and the growth is BROAD, not one bad module**, which is what makes it a
plan-level fact rather than a cleanup task:

```text
features          +4,301   (+10,260 / -5,959 — the hub, churning hard)
items             +2,255
world             +1,121
avatar            +1,040
session           +1,038
construction        +691
character_runtime   +627   (+3,050 / -2,423)
dialog              +537   ⚠ grew even though conversation was carved OUT
```

⇒ **carving one module per session does not keep pace with ordinary feature
work.** That does not make the carves wrong — it means the row cannot be judged
by "did a crate leave this session", and the honest measure is this ratchet.

⚠ **the ratchet is a REAL gate, not advisory** — `compile_ratchet.py` exits 1
by default (verified without a pipe; its own comment says *"Gates that require a
special enforcement flag are too easy to run in advisory mode accidentally"*),
and `scripts/run_tests.py` runs it. So this regression is live, not dormant.

⛔ **DELIBERATELY NOT RE-FROZEN.** The tool offers *"if this is a deliberate
landing, say so and re-freeze"* — this is not a landing anyone declared, and
re-freezing would launder ten thousand lines off the ledger exactly the way a
carve launders doc-link debt off a per-crate one. It stays red until someone
either carves it back down or states the growth as intended.

⚠ two smaller readings from the same run: `critical_path_crates` **12 → 13**,
which is the `conversation` carve's own cost (a new crate adds a hop), and
`ambition_conversation` is **UNPRICED** — it is being charged the population
median, and size predicts compile cost with R² = 0.12, so its seconds are a
placeholder. `python3 scripts/compile_collect.py` would measure it.

> ⭐⭐ **THE NEXT CARVE IS MEASURED, 2026-08-17 — `encounter`, and its one edge is
> NOT REAL.** Doing what this row asks (choose from current measurements) over the
> monolith's fourteen top modules:
>
> ```text
>                      lines   outward `use crate::` edges
>   features           43018   17     ← the hub; not a carve, it IS the monolith
>   character_runtime  13788    3
>   avatar              7717    7
>   boss_encounter      6940    3     (cutscene_trigger, encounter, features)
>   encounter           2168    1     ← ⭐ and the one is a RE-EXPORT
>   schedule            2384    1     (character_runtime)
>   character_sprites   1808    2     (assets, character_roster)
> ```
>
> ⭐ `encounter`'s single edge is `use crate::features::FeatureEcsWorldOverlay`,
> in ONE file — and that type is DEFINED in
> `ambition_platformer2d_shared_tangle::feature_overlay`, BELOW the monolith.
> ⭐ AND IT IS NOT SCHEDULE-PINNED, which is the trap that cost `conversation` a
> whole slice: `EncounterSimulationSchedulePlugin` already owns its registrations
> and already uses a NAMED set (`WaveEncounterDriven`). Nothing to un-chain first.
> ⚠ inward edges are `audio/plugin.rs` and `boss_encounter` — ordinary, and they
> become a dependency on the new crate.

⛔⛔ **THE INSTRUMENT WAS WRONG, AND IT IS THE OPPOSITE OF THE TRAP THE
`conversation` SLICE RECORDED.** That slice learned *"measure `use` statements,
never `crate::` occurrences"* because this repo's doc comments cite paths so
densely that a path-grep measures PROSE. **True there, and it does not generalise:
`conversation` happened to write every edge it had as a `use`.** `encounter`
writes almost none of them that way — its dependencies are **inline
fully-qualified paths in system signatures and plugin bodies**, which a
`use`-grep cannot see at all. Both greps are wrong in one direction each. ⭐ **the
honest instrument is `crate::` paths on NON-COMMENT lines**, which costs one more
`grep -v` and is the only reading that saw this:

```text
module              lines   `use crate::`   crate:: in CODE   ← the honest one
  features          43018       18                25    the hub; it IS the monolith
  character_runtime 13788        3                13
  avatar             7717        7                 9
  boss_encounter     6940        3                 3    ← agrees, genuinely
  construction       5906        3                 8
  items              4985        5                14
  abilities          4881        8                11
  world              4452        6                12
  session            2940       11                18
  schedule           2384        1                 5
  encounter          2168        1                 9    ← ⛔ NINE, not one
  projectile         2127        3                 9
  character_sprites  1808        2                 3
```

⛔ **the `use`-grep undercounts EVERY module in the table and does not undercount
them uniformly** — `boss_encounter` reports honestly (3 = 3) while `items` hides
nine edges and `encounter` hides eight. So the old column could not rank
candidates even relatively. **No carve should be chosen off it again.**

⭐⭐ **BUT THE CORRECTED NUMBER IS ALSO NOT THE VERDICT — MOST OF THOSE NINE
RESOLVE BELOW THE MONOLITH TOO, AND THE FIVE THAT DO NOT ARE THE FINDING.** Every
name `encounter` reaches through a sibling module, chased to its `pub struct` /
`pub fn`:

```text
NOT real (sibling module is a pure re-export of a LOWER crate):
  features::{ChestFeature, EncounterMob, EncounterRewardChest, FeatureId,
             GameplayBannerRequested, Opened}         → ambition_combat
  features::{apply_gameplay_banner_requests, tick_gameplay_banner,
             update_ecs_hazards}                      → ambition_combat
  features::FeatureEcsWorldOverlay                    → shared_tangle
  actor::BodyKinematics                               → platformer2d_core
  actor::PlayerEntity · physics::BaseGravity          → shared_tangle
  schedule::Platformer2dSimulationPhaseMonolith       → shared_tangle
  character_runtime::PreparedCharacterRegistry        → ambition_characters
  rooms::RoomSet                                      → platformer2d_world
  trace::{GameplayTraceBuffer, GameplayTraceEvent}    → ambition_gameplay_trace

REAL — defined in the monolith, and each one blocks the move:
  features::spawn_encounter_mob        features/ecs/spawn/mod.rs:816
  features::EncounterMobSeed           features/ecs/spawn_actors.rs:2086
  features::{clear_encounter_reward_ecs, sync_encounter_reward_chests_ecs}
                                       features/ecs/encounter_rewards.rs:16,41
  features::FeatureWorldOverlaySet     world/overlay.rs:35
  world::gated_lock_walls::sync_authored_gated_lock_walls
                                       world/gated_lock_walls.rs:152
  crate::ActorDiedMessage              lib.rs:156
```

⛔⛔ **THE LOAD-BEARING BLOCKER IS `spawn_encounter_mob` — `drive_wave_encounters`
SPAWNS ACTORS THROUGH THE MONOLITH'S ACTOR CONSTRUCTION PATH** (`systems.rs:335`,
handing it an `EncounterMobSeed`). That is not an ordering nuisance a step-1.5
can name away; it is **actor construction**, which this plan's own Wave G says
leaves LAST, after the outer domains. A wave arena's whole job is *spawn these
characters, watch them die* — so `encounter` cannot precede the spawner it calls.

⛔ **AND IT IS SCHEDULE-PINNED AFTER ALL, IN THREE PLACES**, all in
`EncounterSimulationSchedulePlugin` (`encounter/mod.rs`): (1) an **anonymous
`.chain()`** interleaving `drive_wave_encounters` with two banner systems — the
exact shape step 1.5 deleted, though the mildest instance since both banner
systems are `ambition_combat` and a carved crate could still name them; (2)
`contribute_encounter_lock_walls` ordered `.after(crate::features::FeatureWorldOverlaySet)`,
**a set defined in the monolith** — step 1.5's lesson failing in the exact way it
warned about, *the ordering NAME must live where the module can still reach it*;
(3) the plugin registers `crate::world::gated_lock_walls::sync_authored_gated_lock_walls`,
**a foreign module's system**, deliberately, so the two roads into `gate_solids`
are visible in one place. (2) and (3) are real work; (1) is cosmetic here.

⭐ **AND `encounter` ALREADY HAD ITS CARVE — `crates/ambition_encounter` EXISTS.**
Lifecycle, commands, objectives, participants, timeline, waves, registry, music,
rewards, spec and staging all live there. The 2,168 lines still in the monolith
are what the module's own header calls the residue: *"Facade module … Gameplay-core
keeps the adapters that still touch LDtk, ECS spawning, player/body queries,
feature overlays, banners, save/quest plumbing, and schedule sets."* ⇒ **the
header was accurate and the row proposed re-carving what had already been carved.**
Six of the twelve files are three-line `pub use ambition_encounter::…;` compat
shims. ⚠ the name `ambition_encounter` is therefore TAKEN, which is by itself a
signal a candidate deserves a second look.

⚠⚠ **AND THE INWARD EDGES ARE BACKWARDS FROM WHAT THE ROW SAID** — they are the
laundered ones. `audio/plugin.rs:200` and `boss_encounter` (3 sites) name
`crate::encounter::EncounterMusicRequest`, which is a bare re-export of
`ambition_encounter::music`. So do `music/intent.rs` and `session/reset/mod.rs`
inside the monolith, and `ambition_app`'s + `ambition_demo_mary_o`'s tests through
`ambition_platformer2d::actors::encounter::` — while `ambition_platformer2d_runtime`
and `ambition_content` already name `ambition_encounter` directly. **Two roads to
one type, and the shorter one is the facade.** ⚠ symmetrically,
`encounter/switches.rs:57–59` reaches its OWN `SwitchFeature`/`SwitchOn` back
through `crate::features`, which re-exports them from `crate::encounter`
(`features/mod.rs:130`) — a re-export LOOP.

⇒ ▢ **THE NEXT SLICE HERE IS THE DE-LAUNDERING, NOT A CARVE.** Repoint every
`crate::encounter::EncounterMusicRequest` / `actors::encounter::` consumer at
`ambition_encounter`, close the `switches` self-loop, and delete the six compat
shim files — ~12 sites across four crates, no new crate, no lockfile, no
`critical_path_crates` movement. It removes both inward edges and shrinks the
residue to the adapters that genuinely cannot leave. ⭐ same shape as the LDtk
compat-facade deletion this row already banked: **what it buys is honesty and one
fewer historical path.**

✔✔ **THE DE-LAUNDERING LANDED 2026-08-17 — ALL SIX SHIMS ARE DEAD AND
`encounter/mod.rs` NOW RE-EXPORTS NOTHING IT DOES NOT DEFINE.** The facade went
from **39 exported names (26 of them `ambition_encounter`'s) to 13, all
monolith-owned** — the four adapters (`load_encounter_specs_from_ldtk`,
`contribute_encounter_lock_walls`, the switch table, the wave systems).
Measured with the honest instrument (`crate::` on NON-COMMENT lines):

```text
                         before   after
  encounter → siblings     40       38   sites   (9 distinct modules → 9)
  siblings  → encounter    29        6   sites   ← ⭐ THE RESULT
  ambition_platformer2d::actors::encounter:: consumers
                           11        5   sites   (all 5 monolith-owned)
```

⭐ **THE OUTWARD NUMBER BARELY MOVING IS THE POINT, AND IT IS THE MEASUREMENT
LESSON REPEATING.** De-laundering removes edges that were never real; the two it
dropped (`crate::features::SwitchFeature/SwitchOn`, a module reaching its OWN
types back through the hub) are the whole switch loop, and the distinct-module
count cannot move because `features` still carries 24 other names. **The
direction that changed is INWARD: 29 → 6, and all six of those now resolve to
`encounter/switches.rs`** — types the monolith genuinely defines. Every remaining
`crate::encounter::` in the tree names something encounter OWNS. ⛔ the residue is
2,145 lines, essentially unchanged — this bought honesty, not size, exactly as
the row predicted.

⚠ **the shim deletion was asserted structurally, not by eye:** the same grep for
`mod {events,lifecycle_reexports,music,registry,rewards,spec};` and their `::`
paths returns **21 hits on `HEAD`** and **3 after** — and all three are
`ambition_encounter::spec::default_encounter_reward()`, the owning crate's real
module. A grep that returns nothing only means something if you showed it
returning something first.

⇒ ✔ **AND THE CANDIDATE LIST WAS RE-RANKED — `boss_encounter`'s PRECONDITION IS
CLEARED AND IT IS STILL NOT THE NEXT CARVE.** It reached three sibling modules
(`features`, `cutscene_trigger`, `encounter`); the `encounter` edge was those
three `EncounterMusicRequest` sites and is now **gone — two modules, not three**.
⛔ **but "3 = 3" was never a size, it was a count of MODULES, and chasing them
kills the candidate:** `boss_encounter` carries **155 inward `crate::boss_encounter::`
sites** from its siblings, and its outward edges land on **boss vocabulary the
monolith itself defines inside the `features` hub** — `features/ecs/boss_clusters.rs`
(`BossConfig`, `BossEncounter`, `BossRef`, `boss_is_cleared`), `BossOverrides`
(`features/ecs/spawn_actors.rs:111`) and `sync_boss_reward_chests_ecs`
(`features/ecs/encounter_rewards.rs:98`). ⇒ **the boss's own data model lives in
`features`, so a carve moves the boundary, not the code.** The next slice here is
either that (relocate `boss_clusters` to `boss_encounter`, where it belongs) or a
different candidate entirely — not a `boss_encounter` Cargo.toml.

✔✔ **THE RELOCATION LANDED 2026-08-17 — THE BOSS DATA MODEL NOW LIVES IN
`boss_encounter`, AND THE BIDIRECTIONAL EDGE IS ONE-WAY.** `boss_clusters.rs`
(430 lines) moved to `boss_encounter/clusters.rs`; `BossOverrides` moved out of
`features/ecs/spawn_actors.rs` to sit beside the components it tweaks; and
`sync_boss_reward_chests_ecs` moved into `boss_encounter/rewards.rs` — a file
that existed **only** as a code-free placeholder whose own doc said *"boss
reward-chest sync now lives in `crate::features`"*. ⛔ **no re-export was left
behind**: `features` no longer names any of the ten symbols, and all 55
`features::<boss symbol>` call sites across seven crates were re-pointed.

Measured with the honest instrument — **`crate::` on NON-COMMENT lines, counted
in SITES, both directions**:

```text
                              before   after
  boss_encounter → siblings     51       25   sites  ← ⭐ the result
      of which crate::features   49       21
      new (arrived with the moved code: platformer_runtime, combat)  —  2
  siblings → boss_encounter    155      201   sites  ← ⚠ UP, and correct
  features/mod.rs exported names  280     270          (−10, all boss)
  features tree                43,017  42,339  lines   (−678)
```

⭐⭐ **THE INWARD NUMBER GOING UP IS THE HONEST OUTCOME, NOT A REGRESSION.** Those
46 new `crate::boss_encounter::` sites are the same edges that were already there
reading `crate::features::BossConfig` — the hub was laundering a boss dependency
as a features dependency. Relocation does not delete an edge a caller genuinely
has; **it makes it say whose it is.** The number that had to fall is the outward
one, and it did: 49 → 21.

⭐ **AND NONE OF THE 21 SURVIVORS IS BOSS VOCABULARY.** Twenty of them name types
that live BELOW the monolith and are merely re-exported by the hub —
`BodyKinematics` (`platformer2d_core`), `CenteredAabb` / `ChestFeature` /
`FeatureId` / `Opened` / `FallingChest` / `BossRewardChest` / `GameplayBanner`
(`ambition_combat`), `FeatureSimEntity` (`shared_tangle`). The twenty-first is
`MountDied`, genuinely defined in `features::ecs::mount` — a real cross-domain
message, not laundry. ⇒ **the fable-review 2026-07-15 blocker is cleared**: it
named exactly this file (*"the single blocker is that boss cluster ECS components
live in features/ecs while catalog/behavior/sprites live in boss_encounter"*).

⚠ **NOTHING REFUSED — but two things were checked FOR a refusal and passed.**
(1) `BossOverrides` looked construction-pinned, living in the spawn module; it is
not — it is a plain `Component` of authored tweak DATA, written once at spawn and
read only by `update_boss_encounters` / `sync_boss_encounter_entities`, both in
`boss_encounter`. `spawn_actors.rs` now imports it like any other component it
inserts. (2) `sync_boss_reward_chests_ecs` looked table-pinned, sharing a file
with `sync_encounter_reward_chests_ecs` and `clear_encounter_reward_ecs`; it is
not — those two share only the file's `use super::*`, not a table, and the boss
one has exactly **one** production caller (`boss_encounter::systems`). Its mob
siblings stayed put: their `EncounterMob` wave vocabulary is `encounter`'s, not
the boss's.

⭐ **ONE PRE-EXISTING VIOLATION SURFACED, which is the facade-deletion hazard
AGENTS.md names.** Splitting a grouped import
(`use …monolith::features::{BossClusterRef, FeatureEcsWorldOverlay}`) left
`FeatureEcsWorldOverlay` on its own line and `engine.f2-consumers-use-canonical-crates`
fired — the edge had been hiding inside a mixed brace. Fixed by naming its real
home (`ambition_platformer2d_shared_tangle::feature_overlay`), not by waiving.

⚠ **and two ROLLBACK ORACLE strings had to move with the type**, because they are
`std::any::type_name` text, not paths a compiler checks:
`…::features::ecs::boss_clusters::BossConfig` and
`…::features::ecs::spawn_actors::BossOverrides` → `…::boss_encounter::clusters::*`.
A relocation of a rollback-registered component always owes that edit.

The absence was asserted structurally, not by eye: `boss_clusters` as a path went
**62 sites → 1** (the survivor is a comment in `boss_encounter/mod.rs` recording
where the module came from), and `features::<boss symbol>` went **55 → 0**.
Green: `cargo check --workspace --all-targets`, monolith lib (1,245),
`ambition_app --test app_it`, `ambition_workspace_policy` (34), and
`check_absence_contracts.py --check` (29 of 29).

⇒ ▢ **THIS STILL DID NOT MAKE `boss_encounter` A CARVE, AND THE 201 INWARD SITES
ARE WHY.** The domain now owns its vocabulary, but `features` alone names it 155
times — every boss query filter (`With`/`Without<BossConfig>`), the damage router,
the anim helpers, the save sync, the reset. A carve would have to take
`features/ecs/bosses/` (tick + sync, 1,791 lines) and `features/bosses.rs` with
it, and `features/ecs/damage/boss_hit.rs` needs a `HitEvent` seam first. **That is
the next slice if this candidate is pursued — a second relocation, still not a
Cargo.toml.**

⛔ nothing was committed against the carve itself: no crate, no manifest, no
lockfile moved, so `critical_path_crates` stays at 13 and no baseline was touched.

✔✔✔ **THE CARVE LANDED 2026-08-17 — `crates/ambition_boss_encounter`, and the
paragraph above was WRONG ABOUT WHY.** 7,635 lines left the monolith; the second
relocation it asked for was never needed.

```text
largest_unit_lines   ambition_platformer2d_actor_monolith
                     121,822 → 114,139   (−7,683; still +2,710 over the frozen
                                          111,429, was +10,393 this morning)
critical_path_crates      13 → 13        ⭐ NO new hop
```

⛔⛔ **THE 201 INWARD SITES WERE NEVER A BLOCKER, AND THE DIRECTION ERROR IS THE
FINDING.** An inward site is a CALLER naming the domain; after the carve it spells
`ambition_boss_encounter::` instead of `crate::boss_encounter::` and compiles
unchanged — a rename, not a dependency the departing crate must satisfy. **Only
OUTWARD edges block a carve**, because those are the ones cargo refuses. So
`features/ecs/bosses/` never had to move: it CALLS the boss domain, it is not
called BY it. ⇒ **count both directions, adjudicate on the outward one.**

⭐ **the outward list had TWO real names, and both moved DOWN, not across.**
Measured with the honest instrument (`crate::` on NON-COMMENT lines, in SITES);
each distinct path chased to its `pub struct` / `pub fn`:

```text
module            lines   out sites/mods   in sites/mods   ← the ranking that chose it
  features        42,343     579 / 21        499 / 20   the hub; it IS the monolith
  character_runtime 13,788    87 / 12        288 / 10
  avatar           7,717     164 /  8         85 / 14   ⛔ 15 of 164 are one type in
                                                          character_runtime — pinned
  boss_encounter   7,635      24 /  3        201 / 12   ← ⭐ CHOSEN
  construction     5,906     245 /  6         39 /  4   ⛔ Wave G leaves LAST
  items            4,991      85 / 13         44 /  4
  abilities        4,881     136 / 10         30 /  4   ⛔ calls spawn_runtime_minion
  world            4,452      55 /  9         53 /  4
```

⛔ **`out mods` alone would have picked the wrong one** — `character_sprites` (3)
and `boss_encounter` (3) tie, and `world` (9) looks worse than `avatar` (8) while
being far cleaner. The number that decided it was the outward SITES chased to a
DEFINITION: eleven of `boss_encounter`'s thirteen distinct paths resolved to crates
already BELOW the monolith and the hub was merely re-exporting them
(`BodyKinematics`, `CenteredAabb`, `FeatureId`, `FeatureSimEntity`,
`GameplayBanner`, `ChestFeature`, `Opened`, `FallingChest`, `BossRewardChest`,
`falling_chest::settled_chest_center`). The two that were real:

* `CutsceneTriggerQueue` → `ambition_cutscene`, beside the script format it
  triggers. `crate::cutscene_trigger` is deleted, not re-exported.
* `MountDied` → `ambition_platformer2d_shared_tangle::body`, below BOTH domains
  that share it — the mount coupling WRITES it, the boss crate READS it. Same move
  and same reason as step 1.5 putting `FeatureInteractionSet` there. ⛔ imported
  privately into `features/ecs/mount`, never re-exported, so nothing can keep
  spelling it `features::MountDied`.

⭐ **the ORPHAN RULE adjudicated one more file, exactly as `snapshot_impls.rs`'s own
header promised**: `impl SnapshotCursor for BossEncounter` stopped compiling the
moment the type crossed the crate line and moved to `clusters.rs` with it. The wire
format did NOT change — `rollback-wire-format-is-frozen` reports the same 357 names
and 85 encoded types.

⚠ **the umbrella, not a new edge, is how a demo reaches it.** Naming
`ambition_boss_encounter` directly from `ambition_app` / `demo_mary_o` /
`demo_sanic` reddened three `game.*-umbrella-only` policies, correctly:
`ambition_platformer2d` re-exports every domain crate under a short name for
exactly this. Only `platformer2d_runtime`, `platformer2d_provider`, `sim_view` and
`ambition_content` declare the edge — and the runtime allowlist gained its entry in
the same commit, **the fifth time that list has lagged a runtime dependency**.

⚠ **the ledgers a carve launders, all moved in the same commit:**
`check_doc_link_ratchet.py`'s `CRATES` gained `ambition_boss_encounter` (monolith
109 → 107, new crate 2, total 191 → 192 — the carve is link-neutral);
`capability-footprint-baseline.json` 43 → 44 crates and 16 → 17 never-asked-for,
with the argument written in; three lockfiles; `engine.toml`'s runtime allowlist
plus two now-stale absence strings re-pointed so they still guard something.
⚠ **and one honest cost UP:** `ambition_geometry`'s `worst_edit_cost` goes 48 → 49
crates (+17.5s) — one more compilation unit sits above it.

⇒ ✔✔✔ **THE RATCHET IS GREEN — `largest_unit_lines` 114,139 → 110,929, BELOW
the 111,429 frozen on 2026-08-09 for the first time since it was frozen.**
`critical_path_crates` stayed **13** — no new hop, because no new crate was
made. Landed 2026-08-17.

⛔⛔ **BOTH NAMED CANDIDATES WERE REFUSED, AND THEY WERE REFUSED FOR THE SAME
REASON `encounter` WAS.** Chasing every outward site to its `pub struct` /
`pub fn` — and splitting PRODUCTION from TEST, which no previous measurement on
this row did:

```text
module   lines   real-out PROD   real-out TEST   re-export (not real)
  items   4,991      30 sites         5              49
  world   4,452      11 sites        26              17
```

⭐ **the split is what the earlier column was hiding.** `world` looks far
cleaner than `items` by production sites (11 vs 30) — and 20 of `items`' 30 are
one thing, `ItemPickupPlugin` registering **eighteen `abilities` systems and two
`shrine` systems** into its own sets, which is step 1.5's shape and mechanical to
fix. ⛔ **but both keep a `construction` edge**: `items/pickup/mod.rs` rebuilds a
carried object from `construction::authored_occurrence_request` +
`ActorConstructionParams::GroundItem`, and `world/rooms/{stage,transaction}.rs`
stage actors through `ActorConstructionPlan` / `verify_rig_composition`. That is
**actor construction, which Wave G says leaves LAST** — the identical blocker
(`features::spawn_encounter_mob`) that refused the `encounter` carve this
morning. ⚠ and `world`'s is BIDIRECTIONAL: `construction/mod.rs:47` imports
`crate::world::placements::ActorPlacementContext`.

⇒ **the direction rule holds and gains a corollary: only OUTWARD edges block a
carve, and the one that blocks BOTH remaining outer domains is the same one.
Until `construction` moves, `items` and `world` are not carves.**

⭐⭐ **SO THE SLICE MEASURED ALL FORTY MODULES WITH THAT INSTRUMENT INSTEAD, AND
THE ANSWER WAS NOT A CARVE AT ALL — IT WAS FOUR RELOCATIONS INTO CRATES THAT
ALREADY EXISTED.** Ranking every top-level module by real production outward
edges surfaced a population nobody had counted: **modules with ZERO of them,
whose owning crate is already in the tree.**

```text
                                          real-out   destination
  persistence   1,336  DELETED — dead       0/0      (nothing; see below)
  menu            809  → ambition_menu      0/0      map.rs was already there
  dialog          672  → ambition_conversation 0/0   the dialogue authority
  equipment       388  → ambition_items     0/0      it IS an item
                ─────
                 3,205  measured 3,207 in the monolith's line count
```

⛔⛔ **`persistence` WAS DEAD, and the falsifier is the finding.** Its 1,336
lines were an eleven-line re-export adapter, a 36-line settings facade, and
`settings/model` — 1,289 lines of pause-menu vocabulary. **All eight of its
public names (`SettingsPage`, `SettingsItem`, `SettingsAction`,
`SettingsOutcome`, `DevToggleSnapshot`, `apply_action`, `apply_display_mode`,
`PLAYER_DAMAGE_SLIDER_MAX`) have zero code references anywhere in `crates/` or
`game/`** — every hit outside the module is a doc comment. ⭐ the instrument was
shown WORKING first: the same grep run on `TextureResolutionScale` and
`reconcile_equipment_grants` returns real call sites. The remaining 25
`crate::persistence::settings::…` paths were all re-exports of
`ambition_persistence::settings::{TextureResolutionScale, AudioSettings,
TriggerEdgeState}` and were repointed at their real home.

⭐ **and each destination was chosen by what the crate ALREADY OWNED, not by
where there was room.** `ambition_menu::map` already held `MapMenuState` and the
monolith held the renderer that imported it — the two halves of the Map tab were
a crate apart. `ambition_conversation` already owns the conversation authority
the Yarn runtime is driven by. `ambition_items` already owns the item catalog
`equipment` grants verbs from.

⛔⛔ **AND THE DESTINATION CHOICE IS WHERE THIS SLICE ALMOST WENT WRONG — THREE
OTHER "OBVIOUS" HOMES WERE REFUSED BY THE CRATES THEMSELVES.** `ambition_dialog`
declares itself *"content-free — the host maps `DialogState.active` onto its own
session mode"*, which is precisely what the moved glue does, so it cannot host
it. `ambition_settings_menu` is the *renderer-agnostic* IR and carries no
`bevy`. `ambition_menu`'s manifest says its trimmed bevy feature set is
*"load-bearing for the WHOLE workspace"*. ⇒ **read the destination's stated
contract before moving code into it; a crate that refuses your dependency is
telling you the code does not belong there.** The Map tab passed that test
because its three new edges (`ambition_input`,
`ambition_platformer2d_shared_tangle`, `ambition_platformer2d_world`) widen
nothing: `ambition_menu` already sat downstream of `ambition_platformer2d_core`
through `ambition_ui_nav → ambition_input`, so **no crate joined
`ambition_geometry`'s or `ambition_platformer2d_core`'s rebuild set** — both
`edit_cost` ledgers moved DOWN, not up.

⭐ **two declared dependency EDGES died with the moves, which a line count cannot
see.** The monolith no longer names `ambition_menu` at all, and `bevy_yarnspinner`
+ `yarnspinner` left its manifest entirely (its `ui` feature now only FORWARDS to
`ambition_conversation/ui` + `ambition_dialog/ui`). ⚠ one edge was added on
purpose: `ambition_platformer2d` now re-exports `ambition_conversation as
conversation`, because a game reaches a domain crate through the umbrella —
`ambition_platformer2d::conversation::dialog::YarnBridgePlugin`.

⚠ **the ledgers a relocation launders, all moved in the same commit** — and this
shape launders *better* than a carve does, because there is no new `Cargo.toml`
to remind anyone: `check_doc_link_ratchet.py`'s `CRATES` gained
**`ambition_items` and `ambition_menu`** (monolith 107 → 103; the two
destinations were carrying 5 and 3 unlisted); four lockfiles under `fixtures/`
and `examples/` refreshed; **seven** workspace policies re-pointed rather than
deleted, two of which had to change SIDES —
`game.lib-menu-keeps-map` REQUIRED `src/menu/map` in the monolith and is now
`game.lib-menu-gone` forbidding `src/menu`, and
`engine.actors-settings-surfaces-controls` asserted the persistence facade
re-surfaced `controls` and is now `engine.actors-persistence-facade-gone`
forbidding the whole directory. ⭐ `capability-footprint-baseline.json` needed
NO edit — 44 crates / 17 never-asked-for, unchanged — and
`rollback-wire-format-is-frozen` reports the same **357 names, 85 encoded
types**: nothing that moved was rollback-registered.

Green: `cargo check -p ambition_app --all-targets`, `cargo test --workspace
--lib`, `ambition_app --test app_it` (412), `ambition_demo_smash_app` (32),
`ambition_workspace_policy` (34), `check_absence_contracts.py --check` (29/29).

⇒ ▢ **NEXT, and the row's shape has changed.** The ratchet is green, so the next
slice is not a race against a number. The remaining outer domains are
`audio`+`music` (1,842 lines, ZERO real outward edges either direction, and
**nothing else in the monolith references them** — three `Platformer2dAudioPlugin`
adds and one rollback-oracle string are its whole consumer set) and
`character_roster`/`cutscene`; each wants a home that will accept it, and
`audio` additionally wants the monolith's `audio`/`web_audio` persona features
forwarded to wherever it lands, which is the real cost there. ⛔ **`items` and
`world` are blocked behind `construction` and should not be attempted again
until it moves** — and the `abilities`-registration half of `items` (20 of its 30
production edges) is a step-1.5 slice that can land independently and would
make `items` a genuine candidate the moment Wave G opens.
 Prefer boundaries that improve capability closure, compile isolation,
public API shape or change amplification.

⭐⭐ **MEASURED 2026-08-15, and the headline is a REFUTATION worth more than the
carve it refused.** `conversation` is the strongest candidate by every import
measure — **1,836 lines, zero edges out, zero edges in** — and its own header
claimed *"the carve is a Cargo.toml"*. ⛔ **it is not.**
`features::FeatureInteractionSchedulePlugin` performs every registration it has
and interleaves three of its systems into **ONE anonymous `.chain()`** with the
switch and chest systems, every interleave load-bearing and documented only in
prose at the call site. ⇒ **a module with zero inward imports can still be pinned
by the SCHEDULE.** Step 1.5 is therefore a `ConversationPlugin` owning those
registrations and stating the cross-domain order as **named sets** — a
simulation-ordering change that wants a session able to run the suite.

✔✔ **STEP 1.5 LANDED 2026-08-16 (`bc187bc98`) — THE CHAIN IS GONE AND THE CARVE
IS NOW GENUINELY A CARGO.TOML.** `FeatureInteractionSchedulePlugin` held ONE
anonymous `.chain()` of **ten** systems across four domains (this row said
eleven; it was ten), and the whole cross-domain ordering contract was adjacency
in that tuple plus five prose comments. `FeatureInteractionSet` now names the
phases — `NarrativeIntake · Actuate · Continuity · CutBarkCast · HoldProjection ·
WorldObjects · SwitchIndex` — and every rationale lives on the variant it
explains rather than beside the system it happened to precede. It follows the
existing `ProgressionSet` / `PlayerInputSet` template rather than inventing a
shape: the phase owner chains the SET LIST once, each domain states only which
phase it is in.

⭐⭐⭐ **AND THE PLACEMENT IS THE TRANSFERABLE LESSON — the set vocabulary lives
in `shared_tangle`, BELOW the monolith, on purpose.** A set enum defined in
`features` would have re-pinned `conversation` by the schedule the moment it
stopped importing `features` — *the same bug, one level up.* ⇒ **when you name an
ordering so a module can leave, the NAME has to live somewhere the module can
still reach after it has left.**

⭐⭐ **RE-MEASURED 2026-08-17: THE CARVE IS STILL READY, and the numbers are
now exact.**

```text
conversation -> monolith    0 real `use` statements      ⇒ the new crate need
                                                          not depend on the one
                                                          it leaves. THIS is the
                                                          direction that decides
                                                          a carve.
monolith -> conversation    1 `use` (features/ecs/interact.rs:19,
                            `DialogueDispatch`) + 35 inline paths
                                                        ⇒ ordinary, and becomes
                                                          the monolith depending
                                                          on a crate.
size                        2,734 lines = ~2,023 code + 711 test
                            (the row said 1,836; it grew, it did not rot)
```

⛔⛔ **AND A MEASUREMENT TRAP WORTH MORE THAN THE NUMBERS.** `grep -r "crate::"`
over this module reports edges to `participant_seat`, `features`,
`character_runtime`, `items` and `dialog` — and **every single one is a DOC
COMMENT**. This repository's `//!` and `///` blocks cite paths so heavily that a
path-grep measures PROSE, not dependency. I nearly filed this row as stale on
that reading. ⇒ **measure `use` statements, never `crate::` occurrences.**

⚠ the remaining cost is the one the repo already knows: a new crate is a new dep
edge, which is FIVE lockfiles and the contracts job — see
a new dep edge fails the contracts job, and one of the five lockfiles is
invisible to `git status`.

⭐ **`ConversationPlugin` owns** `ActiveConversation`, the `ConversationCutBark`
port channel, the `ConversationEnded` ledger install, the `Update` presentation
pair and its three sim systems. ⚠ **only ONE of the seven `NarrativeInputPlugin`
installs moved, and that is the seam rather than a shortfall**: a ledger payload
belongs to whoever CONSUMES it — three are `features` types a carved crate could
not name, three more are applied by `features::bus` and `items::narrative`.
**Conversation provides the mechanism, not the vocabulary.**

✔ **four schedule-graph tests assert the edges AS THE PLUGIN COMPOSES THEM** —
set-to-set dependencies, nesting in the containing phase, each system's
membership, and that nothing sits in the phase outside a named set — all four
probe-falsified by breaking the composition rather than reasoned about. ⭐ that
is the shape that beats a hand-listed chain, which pins the function and not the
wiring.

✔✔ **STEP 2 LANDED 2026-08-17 — `crates/ambition_conversation` EXISTS AND THE
CARVE REALLY WAS A CARGO.TOML.** The measurements held on re-verification: zero
`use crate::` in the whole module, ten files, 2,734 lines. ⭐ **not one line
inside the moved code changed shape** — `use super::authority::…` resolves to the
crate root exactly as it resolved to the parent module, so every internal path
survived `mod.rs` → `lib.rs` untouched. The single edit inside the carved code
was a `warn!` `target:` string that still spelled the monolith. Everything else
was manifests and `crate::conversation::` → `ambition_conversation::` at the
CALLERS: seven files in the monolith, two in `ambition_platformer2d_runtime`, six
in `ambition_content`. **Name from the module's own header, which proposed it in
2026-08-07**; the `ambition_platformer2d_*` prefix belongs to crates that are
platformer-shaped, and conversation continuity is not.

⭐ **NOTHING had to move to `shared_tangle` first.** Step 1.5 had already put the
only shared vocabulary the crate reaches upward for — `FeatureInteractionSet`,
`SimScheduleExt`, `SimId` — below the monolith, which is precisely the lesson it
was recorded for. Everything else the crate names (`ambition_characters`,
`ambition_combat`, `ambition_dialog`, `ambition_input`, `ambition_interaction`,
`ambition_platformer2d_core`, `ambition_time`) was already beneath it. The carve
found no second pin.

⛔⛔ **AND IT COST A CRITICAL-PATH HOP — `critical_path_crates` 12 → 13, which
this plan predicted would stay at 12.** MEASURED, not inferred: recomputing the
first-party height with `ambition_conversation` folded back into the monolith
gives 12 and with it carved gives 13. The lengthened chain is `conversation →
ambition_dialog → ambition_ui_nav → ambition_input → ambition_platformer2d_core →
ambition_geometry` — inserting a layer under `ambition_dialog` pushed that whole
tail down one hop. ⭐ **this is exactly the regression that number is guarded for:
every size metric can improve while the serial chain, and so the wall clock, gets
worse.** ⚠ read it in HOPS — rustc releases a dependent at the predecessor's
`rmeta`, so a chain edge serialises only the frontend, and this repo has already
measured a naive chain-of-durations overshooting a real build by 2.2x.
⚠ **the ratchet baseline was deliberately NOT re-frozen**: it is frozen at
`208cf8acf937` (2026-08-09) and reports NINE findings, eight of them eight days
of unrelated growth. Re-freezing under a carve commit would launder them.
⚠ **and this crate's seconds are a PLACEHOLDER** — it is unpriced, estimated at
the population median 2.9059 ms/line, and size predicts compile cost with
R² = 0.12. `scripts/compile_collect.py` is what makes it real.

⇒ **the five-lockfile / contracts cost arrived exactly as documented and cost
nothing to pay.** Root, `fixtures/minimal_game` and `examples/capability_demo`
changed and are committed; `examples/portal_tutorial` did not move (predicted);
`fixtures/external_consumer` was re-resolved and is gitignored, so it is
correctly absent from the diff. `capability-footprint-may-not-grow` went RED on
`ambition_conversation entered the consumer's closure` and the baseline moved in
the same commit — **15 → 16 unwanted crates, and the carve did not CAUSE that, it
NAMED it**: the same code was already linked inside the monolith under a name the
counter could not see. `ambition_workspace_policy` went red on
`engine.runtime-manifest-allow` — **the fourth time that list has lagged a runtime
dep by one file**, which its own comments record three times already.

⭐ **the "is it really a carve" check is CARGO ITSELF, and the probe is the
finding.** Adding `[dependencies] ambition_platformer2d_actor_monolith` back to
the new crate does not fail a policy — **cargo refuses to resolve the workspace
at all**, naming the cycle. ⛔ so a denylist for that edge would be a check that
cannot fail. ⭐⭐ **but `[dev-dependencies]` is a real hole and cargo allows it on
purpose** (the monolith relies on that itself), so one test reaching back for a
fixture would rebuild the whole monolith to build this crate's tests. ONE policy
guards that — `engine.conversation-does-not-depend-on-what-it-left`, probe-
falsified by adding the dev-dep and watching it fire. The source-text twin that
was drafted alongside it was DELETED before landing: it guarded the same hole
twice, and this repo's own rule is that a guard with no failure behind it is
ceremony.

⇒ ▢ **STEP 3 IS STILL OPEN AND IS NOT WHAT IT LOOKED LIKE.** `ambition_dialog`
does NOT become a `[dev-dependency]` of the monolith, because `dialog.rs` (135
lines, `ui`-gated) is still a production namer. ⭐ that file is the next slice and
it is clean — **zero `crate::` edges**, naming only `ambition_dialog`,
`ambition_input` and `shared_tangle`. Its cost is a `ui` feature on the carved
crate forwarding to `ambition_dialog/ui`, which is why it stayed out of a move
that was otherwise a manifest. ⛔ and it buys no footprint either: the monolith's
edge to `ambition_conversation` is unconditional, so `ambition_dialog` and
`ambition_ui_nav` reach a movement-only game regardless. Shedding the capability
needs OPTIONALITY at the monolith's edge, not another move.

⇒ **every other leaf is NOT YET on this plan's own scorecard:** `menu` is the
sole namer of `ambition_menu`, but the crate also arrives through render and the
host, so the consumer footprint stays flat — the lesson `ambition_ui_nav` already
paid for. `affordances`, `gravity`, `snapshot_impls` and `action_scheme` remove
**no Cargo edge at all.**

✔ **one real deletion landed with the measurement: the LDtk compat facade.** Six
lines of blanket `pub use` whose own doc named the plan it waited on. ⚠ **the plan
said one consumer remained; it was EIGHT** — a `| head`-truncated grep had hidden
seven, which is the documented absence-grep footgun. ⭐ **and the facade's
optionality was a FICTION**: `game_assets` takes a `WorldManifest` — LDtk
vocabulary — in an **ungated public signature**, compiling only by reaching the
type through the monolith's re-export, whose own LDtk edge is unconditional. The
optionality was purchased by laundering, so declaring the dependency is what made
the facade deletable. ⛔ **no measurement moves** (`closure_size` 42,
`never_asked_for` 15) and that was predicted, not discovered — what the slice buys
is honesty and one fewer historical path.

✔ **two policy entries went with it**, having become checks that cannot fail:
`engine.portal-core-no-content-roster` forbade two `…actor_monolith::ldtk_world`
spellings that can no longer exist. Replaced by the one live name, which neither
portal crate names, so the rule stays green and now guards something.

⚠ **the measurement itself needed a correction to be trustworthy:** the monolith's
44-module graph had to be re-derived **with comments stripped**, because log
targets and doc citations otherwise score `ambition_platformer2d` — a crate
**above** the monolith — as a production edge from ten modules.

⭐ **THE MEASUREMENT, TAKEN 2026-08-14 — and it says the opposite of what size
says.** 117k lines across 33 top-level modules. Counting module-to-module
references (`crate::<mod>` from inside a sibling), the entanglement ranks:

| module | ← depended on by | → depends on | note |
|---|---|---|---|
| `features` | **19** | 18 | 41.7k lines, and the LAST thing to carve |
| `control` | 12 | 4 | |
| `avatar` / `character_runtime` / `boss_encounter` | 11 | 9 / 9 / 2 | |
| `schedule` | 10 | 6 | |
| `session` | 7 | 14 | highest OUTBOUND — a composition root, not a domain |

⛔ **so `features/` is the wrong place to start even though it is a third of the
crate** — it is the most entangled module in both directions, and carving by size
would begin exactly where change amplification is worst.

⛔⛔ **BUT THE "SIX LEAVES WITH ZERO INBOUND EDGES" HALF OF THIS MEASUREMENT WAS
WRONG, AND THE WAY IT WAS WRONG IS THE LESSON (verified 2026-08-14).** A
`crate::<mod>` grep is a LITERAL text measure, and it has two blind spots that
both produced false leaves:

- **`equipment` is not a leaf at all** — `action_scheme.rs` names
  `crate::equipment::reconcile_equipment_grants` three times, registering it and
  ordering two systems `.after` it. The grep simply missed it.
- **`dev` is not a leaf either** — its sibling consumers reach it as
  `crate::trace::`, through `pub use dev::trace;`. ⭐ **a RE-EXPORT ALIAS makes a
  module invisible to a name-based coupling census.**

⇒ **all six "leaves" are LIVE**, and four are live only from OUTSIDE the crate
(`gravity`, `affordances`, `menu`, `quest` — `menu` has zero sibling consumers and
is reached only by the app, runtime and shell host). ⚠ so the crate-placement
question about `menu` and `quest` living in an *actor* monolith is real, but it is
a PLACEMENT question, not a dead-code one. `character_roster` is **vestigial by
design**: a private `mod` with `pub(crate) fn catalog()`, consumed by five files
that are all tests, and its own doc says so. Leave it.

⇒ **PROMOTED OUT OF THIS ROW, because both are cross-crate and neither is a
carve:** the two write-only findings below are the highest-value D33 work
available, and both are DELETIONS rather than moves — which is the only kind of
decomposition progress this repository counts.

⭐⭐ **the genuine change-amplification finds were elsewhere, and they are large:**

1. ⛔ **`PlayerAffordances` is WRITE-ONLY.** `compute_player_affordances`
   recomputes it every sim tick and **nothing in the workspace reads it**. Its doc
   says *"the HUD reads it to label each on-screen button"* — but
   `ambition_touch_input` states in its own comment that labels *"now come from
   the CONTROLLED subject's action scheme via the `ControlPrompt` read-model, not
   the fixed smash-vocabulary affordance table"*. ⭐ **the consumer migrated and
   the producer stayed.** ~1200 lines are recoverable, plus three
   `declare_rollback_derived_resource` calls — so it owes a rollback schema
   re-baseline. ⚠ `interactable_proximity` and `InteractVariant` must SURVIVE for
   the portal adapter.
2. ⛔ **`GravityFlipSwitch` is spawned in exactly one place: its own unit test.**
   `gravity/plugin.rs` says `gravity_flip_switch_system` is *"intentionally NOT
   registered"* because nothing spawns the switch — yet the component is still
   rollback-registered by the runtime, queried by `ambition_sim_view::facts`, and
   given a visual by `ambition_render`. **Three crates carrying a mechanic that
   cannot occur.**

✔ **and four genuinely dead items were deleted** (each verified zero-consumer
first): `ambition_persistence::quest::registry::push_room_entered_quest_event_for_room`
— the abandoned half of a fork, whose first parameter is not a `SystemParam`, **so
it could never have been registered as a Bevy system**; the monolith `quest`
module's whole-namespace `pub use ambition_persistence::quest::*`;
`menu::map::ui::spawn_map_menu`, whose sibling's doc cited a process-resident
direct-entry host that does not exist; and `affordances::variants::IconId`, an
**uninhabited** `enum IconId {}` behind a default method that could only ever
return `None`.

⚠ **three caveats before anyone acts on this table.**⚠ **three caveats before anyone acts on this table.** (1) "No sibling depends on
it" is not "nothing depends on it" — these are `pub mod`s and the app or another
crate may consume them; check outward edges before moving anything. (2) A move to
a workspace crate must clear the ORPHAN RULE, which is what actually adjudicates
placement. (3) ⛔ **AGENTS.md says this crate is not awaiting a size-driven
carve** — the value of a carve is capability closure, compile isolation and public
API shape, so a leaf that nothing consumes may simply be DEAD, and measuring that
is cheaper than moving it.

- ◐ **D71/D92 — Finish the real room-transition transaction path.** ⭐ **THE
  CENSUS IS CLOSED (2026-08-14): the shipped rollback host now opens a readiness
  transaction on every room change — 21 changes / 21 transactions, was 24 / 0.**
  One semantic `RoomTransitionIntent`, all four origins on it,
  `RoomTransitionRequested` DELETED, readiness moved host-side, the confirmed
  commit gated on the same transaction. ⭐ **AND THE TWO PATHS ARE NOW ONE**
  (2026-08-14): `RoomTransitionApplication` is the only implementation of *"put
  this RECORDED subject in this PREPARED room"*, reached by the eager host as a
  `SystemParam` and by the confirmed host through a `SystemState` on `&mut
  World`. `load_room` (24 params), `apply_room_transition_resets`,
  `RoomConstructionPlan::apply_to_world` and `resolve_transition_subject` are
  DELETED; the eager system went from 16 `SystemParam`s to 2. The fork had
  already cost a live defect — the shipped rollback host never cleared room
  carryover, so a door carried enemy projectiles and a modified gravity into the
  next room — measured RED and now green. What remains under this row is the
  CANCELLATION asymmetry, the prefetch/latency MEASUREMENT, and predicted-intent
  readiness.

⭐⭐ **2026-08-14: THE SHIPPED HOST HAD NO INSTRUMENT, AND THAT IS ALSO JON'S
MAGENTA FLASH.** `RoomTransitionLoadPhase::Committed` has exactly one writer (the
EAGER commit) and one reader (the presentation adapter's retirement gate). The
confirmed route never set it — `retire_committed_room_transition` nulled `active`
in `PreUpdate`, so the adapter's next `Update` took its *"no active transition"*
teardown branch. **Three things live behind that gate**, and the shipped game got
none of them: the `UnclaimedFeatureViews` **settle wait** (the cover comes down
when the room has been DRAWN, not one frame after it was built), `minimum_visible`
(the anti-strobe floor), and `RoomTransitionTelemetry::record` — **the only site
computing `request_to_ready`, `asset_wait`, `commit_to_first_target_frame` and
`prefetch_hit`, with ZERO samples on the shipped host.**

⇒ **so item 2 could not be answered with numbers because the instrument was never
reachable** — and the right move was to make it reachable rather than to
manufacture a measurement. ⭐ **this is very likely Jon's open observation
*"changing rooms flashes magenta squares for a brief moment"*** (2026-07-30): its
2026-08-09 fix was protecting only the fixed-tick host. ⚠ **the player-visible
half is INFERRED, not seen** — the code path is established; a `capture_scene`
through the Hall door, before and after, is what would confirm it.

⭐ **AND THE MEASUREMENT, TAKEN 2026-08-14 NOW THAT THE PHASES ARE READABLE**
(`cargo test -p ambition_app --test app_it -- hall_transition_cover --nocapture`,
the Hall door — the worst case in the game):

```text
preflight = 1.676ms   manifest = 14.687ms   barrier = (0 settled, 164 total)   prefetch_hit = false
```

⇒ **the transition is ASSET-BOUND, not construction-bound, and by an order of
magnitude.** Construction preflight is 1.7ms; the asset manifest is 14.7ms; and at
commit **zero of 164 assets have settled**, so the entire remaining wait is
loading. ⛔ **so do not optimise construction** — the 1.7ms is not where the door
feels slow. ⚠ `prefetch_hit = false` for the Hall is EXPECTED and is not a defect
to fix: the budget of 4 neighbours is deliberate, and unbounded hub prefetch was
measured on 2026-07-30 at p99 1372ms frames and 1803 MB resident images. ⭐ **a
hub is not idle time, and the door's wait is COVERED by the load foreground** —
that sentence is the correction to an earlier confident analysis of mine that said
the opposite.

⇒ **cancellation, per route — and the asymmetry is real:**

- *rollback rewinds past the request* — **refused by construction, correctly.**
  `ConfirmedRoomTransitionIntent::get()` filters on confirmed frames and GGRS never
  rewinds one, so no transaction can open against a rewindable intent.
- *load fails* — handled asymmetrically but deliberately (headless retires; a
  windowed host keeps it resident to retry).
- *supersession / stale epoch / session change* — handled.
- ⛔ *the player presses Cancel* — **MISSING, and the affordance LIES.** Cancel is
  Retry wearing another name: it drops the transaction and leaves the INTENT
  pending, so `begin` reopens the identical crossing next frame — Escape during a
  Hall load *restarts* it, discarding a prepared plan and manifest. It cannot clear
  the intent from `Update` (rollback state); the deterministic channel for a player
  intent is the input stream the sim reads. ⚠ that carries a product decision, so
  it is written at the decision site rather than improvised.
- ▣ *void crossing (recorded body gone)* — **FIXED 2026-08-16.** The eager host
  now gives this the same terminal meaning as `CommitOutcome::Cancelled`: consume
  the exact pending intent and retire its transaction instead of reopening an
  impossible crossing forever. Death closes the adjacent fixed-tick race too:
  `open_death_interlude` retracts the dead body's crossing, the detector excludes
  `OutOfPlay` bodies so it cannot refill the slot later in the same tick, and the
  eager loader retires the now-orphaned transaction. Rollback hosts deliberately
  do not infer cancellation from speculative intent absence.

⚠ **one more finding, unactioned:** `RoomTransitionLoadState` and
`PendingLifecycleCommit` are plain `init_resource`, **not experience-scoped**. The
Quit route retires the transaction so the common path is covered, but a
quit-to-home that happens before the load foreground exists was not traced.
⭐ experience-scoped state is `app.experience_owns(..)` — and note the trap:
`releasing` a resource that is read as a plain `Res` PANICS; the reset verb is
`resetting`.

⭐ **the shape, stated once:** `begin` opens a transaction on a confirmed intent,
and **only a successful commit or a void subject can close the INTENT.** Every
other "abandon" closes the TRANSACTION, which the still-pending intent immediately
reopens. ⇒ the convergence is the outcome enum, not another cancel path.

Use [`engine/room-transition-loading.md`](engine/room-transition-loading.md).
Exercise a real movement-kernel → loading-zone → readiness/commit path, keep
rollback-host transitions on the same transaction, and close only currently
reproduced provenance/carry/P2P gaps. This is also prerequisite architecture for
D116's eventual different-room participants.

⭐ **the gap was re-measured and RE-DESCRIBED on 2026-08-14, and the description
was the part that was wrong.** The census still holds (fixed-tick 11/11/0,
rollback 24/**0**/24, shipped host `ConfirmedFrameBoundary present=true`), but
the row claimed the rollback host *"bypasses the canonical construction plan"* —
it does not. `commit_transition` prepares and applies the same
`RoomConstructionPlan` and reuses `validated_spawn`. What the shipped route never
runs is the READINESS transaction: asset-readiness authorization, the
presentation cover, the unpresented-failure state, and prefetch accounting. ⛔ an
agent acting on the old sentence would hunt a second constructor that is not
there and leave the real difference — the shipped game changes rooms with no
cover and no failure reporting — untouched.

⛔⛔ **AND THE PLAN'S OWN "DELETE `RoomTransitionRequested`" SENTENCE WAS FALSE
TOO; corrected 2026-08-14 with a production census.** Loading-zone detection is
ONE of FOUR writers — the others are checkpoint resume (`shrine.rs`), Mary-O's
level-completion flag, and the loading UI's Retry. Migrating detection and
deleting the type breaks all three. ⭐ the census also handed the slice its real
prize: the MESSAGE cannot name its subject, so the commit re-resolves
`ControlledSubject`-or-primary a frame later, while `LifecycleIntent::Transition`
already records `subject: SimId` at detection. The richer contract wins, all four
origins move onto it, and the dedup key becomes `(subject, target_room, arrival,
edge_exit)` — ⛔ NOT target-room-only, which would collapse two doors into one
room at different arrivals.

- ▣ **D121 — The browser ran a DIFFERENT application, and the source already
  said this would happen.** LANDED 2026-08-14; **HUMAN-CONFIRMED 2026-08-15.**

See [`engine/web-platform-parity.md`](engine/web-platform-parity.md) — **four
separate defects, not one.** (1) `run_web()` hand-spelled the composition and its
copy lacked `AmbitionShellHosted` / the shell host / an initial route /
`install_ambition_shell_visuals` ⇒ blank canvas; now ONE
`compose_ambition_visible_game` both hosts call, and the third hand-spelled
composition is deleted. (2) The browser registered NO `game://` source, so every
`game://worlds/*.ldtk` resolved through a source that did not exist. (3)
`--served` published ONE implementation crate's `assets/` — measured, the served
tree had **no `worlds/` directory at all**; it now consumes
`package_asset_guard.py compose`, the same seam Android and Steam Deck use, and
names no crate. (4) `index.html` claimed a keyboard capture the app never
requests and never cleared it. Pinned by a composition contract (+ its
uncomposed-App poison) and an `AssetId` platform-parity audit over the production
manifest: **967/967 entries name the same file on both platforms.** ⚠ `.meta`
404s were NOT the bug and were deliberately left alone — no `.meta` file exists
under either root and none is expected. ⚠ the embedded-assets `web` persona is
still unaudited for defect (3).

⭐ **JON RAN THE REAL SERVED BUILD, 2026-08-15.** Verbatim: the browser boots and
visibly runs Ambition; the shell and launcher are visible and functional; served
asset publication works in the browser; arrow keys navigate menus; a gamepad
navigates menus. This objective is CLOSED on a human browser, which is what it
said it needed. What the same session found is NOT this objective and must not be
folded back into it: gameplay movement was dead (D123, fixed and awaiting
retest), Hall of Characters appeared to stick at 99%, and the opening music
crackled and then audibly "caught up" while startup was heavy — all three moved
to D124.

- ▣ **D123 — Gameplay input was owned by a DEVELOPER INSTRUMENT, so the shipped
  browser could not play.** LANDED 2026-08-15; ⛔ awaiting Jon's browser retest.

⭐ **the symptom made no sense until you knew where the latch lived**: arrows and
a gamepad both navigated menus, and neither moved the character. Not a keycode
question — the same gamepad failed, and menus prove device input reached
leafwing. `ControlFrameLatch`, the primary device→tick bridge, was installed by
`dev::rollback_observatory` (behind `dev_tools`), and `HostInputBindingsPlugin`
skipped it under GGRS *because* the observatory owned it. Desktop-dev enables
`dev_tools`; the web persona does not. So the browser had a live GGRS session,
live leafwing actions and seat latches, with no primary latch — and
`capture_latched_local_input` takes it as `Option`, where absent means "nobody
feeds me" and it declines to publish. Seat zero told the simulation the player
was holding nothing, every tick, in silence. Menus were unaffected because menu
frames never enter the session.

⛔ **A DEVELOPER INSTRUMENT MAY NEVER BE LOAD-BEARING FOR GAMEPLAY.** The device
host owns the bridge now, in the same arm as the seat latches; the observatory's
copy is deleted, so there is nothing to double. Pinned in
`ambition_platformer2d_host`, which cannot depend on `ambition_app` and therefore
cannot borrow an observatory — a GGRS host assembled there is the shape the
browser ships — with a frame-stepped poison so the claim is not a tautology.
`web_persona_boot` measures the real persona and fails on a latch that is missing
OR unfed (an accumulator left behind reproduces the bug with the resource
present): `primary device latch = false` → `device_seen: true`.

⛔ **THE ACCEPTANCE IS A HUMAN IN A BROWSER, and nothing else closes it**: arrows
navigate menus AND move the body; a gamepad navigates menus AND moves the body;
`dev_tools` still absent from the web persona.

- ⏸ **D124 — What the browser exposed. BOUNDED AND RESTING at Jon's direction.**

Plan: [`engine/portable-preparation-and-load-explainability.md`](engine/portable-preparation-and-load-explainability.md).
⚠ **that link was missing until 2026-08-15**, so D124's own 484-line plan was
reachable from nothing — the row and the plan existed and did not know about each
other. ⭐ its frame is **portability, not "optimize wasm"**: Brotli, `wasm-opt`,
AudioWorklets and cache headers are measurements, not this campaign.

⭐ **the harvest was a CONTRACT, not an optimisation: asset loaded ≠ CPU resident
≠ GPU resident.** `texture_is_ready` routes on `AssetServer::get_load_state` —
`Some` means ask the server, `None` means the handle is main-world-owned so ask
`Assets<Image>` for presence — and three systems dropped `Assets<Image>`
entirely. The load barrier explains itself (`asset_stall_report`) and phase
timings are portable (`bevy::platform::time::Instant`, because `Time<Real>`
advances once per frame and so measures zero within one).

⛔ **DO NOT RESUME THIS AS A PERFORMANCE CAMPAIGN.** Jon, 2026-08-14: the browser
is an architecture TEST FIXTURE while the engine is decomposed; it does not decide
what gets built next. ⭐ **the test for any tempting task: would we want this
abstraction if the web target disappeared tomorrow?** Semantic asset readiness,
cross-platform phase telemetry, canonical asset publication, host-owned input and
an explainable load barrier pass it. Brotli, wasm audio scheduling, Hall
streaming, a generic residency scheduler and byte shaving do not.

⛔⛔ **and the one change this row must NOT take**: making sprite sheets
`RENDER_WORLD`-only. **Seven** main-world `Assets<Image>` readers exist and
**four use PRESENCE as their readiness signal**, so the flag would turn
"successfully uploaded" into "never loaded" forever and characters would vanish
the moment their textures arrived. ⚠ that was one commit away, caught by review,
and the mistake was a census that counted one reader.

⇒ blocked on Jon's browser retests: does Hall of Characters leave 99%, and does
the opening music still crackle. Case file archived at
[`../archive/planning-superseded/2026-08-14/d124-browser-exposed-preparation.md`](../archive/planning-superseded/2026-08-14/d124-browser-exposed-preparation.md).

- ✔ **D120 — A platform capability is enabled beside the DEPENDENCY that needs
  it, not at the app. CLOSED 2026-08-14; the rule survives, the row does not.**

⭐ **THE RULE, which is the whole point of the row:** when a new target-specific
need appears, **enable it in the crate that DECLARES the dependency, and let the
app forward a semantic capability**. A future consumer of an Ambition runtime
crate should be able to ask for browser support without knowing what that crate
depends on.

✔ verified at HEAD: `ambition_platformer2d_runtime` — which owns `bevy_ggrs` —
declares `web_platform = ["bevy_ggrs/wasm-bindgen"]`, and the app's `web` and
`web_served_assets` personas forward into it. The wrong-half fix
(`getrandom_02` declared at the app) is deleted.

⚠ **`getrandom_03` / `getrandom_04` still sit at the app and that is CORRECT** —
their owners publish no forwarding feature, so the app IS their nearest owner.
They read exactly like the deleted line, which is why the difference is recorded
in `game/ambition_app/Cargo.toml` beside them rather than only here.

Case file: [`../archive/planning-superseded/2026-08-14/d120-platform-capability-placement.md`](../archive/planning-superseded/2026-08-14/d120-platform-capability-placement.md).

- ⏸ **D119 — The archived-work recovery is DONE; what is left is Jon's to decide.**

✔ **every item archived mid-flight on 2026-08-13 was re-measured against HEAD, and
the pattern is the lesson:** each one that looked open had been closed by a
DIFFERENT campaign deleting the thing it was waiting on. ⭐ **nobody re-read the
item after the road it depended on disappeared** — which is this ledger's oldest
failure mode arriving on schedule, and the reason its standing rule is *grep for
the thing a row says is missing before working it.* Measurement record archived at
[`../archive/planning-superseded/2026-08-14/d119-archive-recovery.md`](../archive/planning-superseded/2026-08-14/d119-archive-recovery.md).

⚠ **TWO THINGS SURVIVE, both Jon's, neither blocking:**

1. **Three of the run's thirteen goal checks CANNOT FAIL.** Checks `[0]`, `[4]` and
   `[5]` in `.goal/active.json` grep `authority-convergance-campaign-2026-08-13.md`,
   `overnight-campaign-2026-08-11.md` and `character-template-architecture-2026-08-10.md`
   — all deleted in `5e382342d` when AC7 closed. `grep` on a missing path exits 2,
   `!` inverts it, the check reports satisfied. ✔ the goal PREAMBLE was rewritten
   2026-08-14 on Jon's instruction to scope all of `docs/planning` and route by
   document ROLE rather than filename, which fixes the part that actually
   misdirected sessions. ⛔ **the checks themselves were deliberately NOT edited by
   the agent they judge** — quietly rewriting your own success criteria is not a
   repair.
2. ✔ **DECIDED 2026-08-14: `WornCharacter` STAYS. The `CharacterIdentity` rename
   is REJECTED.** ⛔ **and it is no longer a harmless sed, because the world-instance
   architecture has given "identity" a precise meaning.** `WornCharacter(CharacterId)`
   answers *"which authored character template does this body currently
   instantiate?"* It does NOT answer *"which unique runtime occurrence is this?"* —
   **`SimId` answers that**, and D125 exists to make exactly that distinction
   rigorous. Naming the component `CharacterIdentity` would collapse

   ```text
   authored character definition identity  ≠  runtime occurrence identity
   ```

   at the precise moment the engine is separating them. ⭐ it must stay legal for
   two bodies to hold `WornCharacter(Fia)` at once even though Ambition's content
   policy intends one Fia, and for `RecharacterizeBody` to change the worn
   character while the runtime occurrence stays the same. Both facts are exactly
   what "worn" says and "identity" denies.

   ⚠ if the "worn" metaphor is ever disliked, **`CharacterForm` or
   `CharacterTemplateRef`** preserve the distinction; `CharacterIdentity` is
   specifically the name to avoid. ⇒ the residue is not a rename but **stale
   player-centric prose around the type**, which should be cleaned up in place.

- ⏸ **D64 — Mary-O / LDtk authoring. RESTING as a successful ACCEPTANCE
  BASELINE, not a running campaign (2026-08-15).**

✔✔ **the end-to-end authoring acceptance landed.** A new level can now be
created through LDtk **without adding ordinary Rust level registration**: new
authored rooms need no Rust routing merely to exist, the demo shell reviews any
authored room, room destinations are authored, warp tubes scope to the active
room instead of hard-coding 1-1, one shared `ldtk_entity_contract.json` is
consumed by both the Rust prover and the Python validator so validation refuses
exactly what the real converter refuses, enemy facing has editor vocabulary, a
ratchet guards against deleting authored levels, and the destructive one-shot
world bootstrap is gone. That is an Engine 1.0 milestone.

⛔ **do not keep adding Mary-O tooling because the lane existed.** The next LDtk
improvement must come from **actual content-authoring friction**: author or
revise a room, hit a real editor/semantic limitation, fix *that* generically.
⛔ do not assign a worker to generic LDtk cleanup.

Preserved architectural rules: `.ldtk` is authoritative spatial source · tools
edit it additively and in place · destructive bootstrap regeneration must not
return · Rust and Python validation must agree about what runtime conversion
accepts · provider/game vocabulary stays discoverable · native editor
fields/enums/entity references where practical · game-specific semantics stay
provider-owned rather than growing a central engine taxonomy.

Residue below is kept as evidence, not as dispatchable work.

The multi-coin block pays the purse and, since 2026-08-14, draws the coin
(`VfxMessage::CoinPop`). Do not reopen old restart or block reports without a
reproduction. Product questions stay in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

⛔⛔ **I FILED LIVES AS UNSTARTED AND IT HAD ALREADY LANDED (2026-08-14) — my
error, and the exact rule this ledger states at the top.** `MaryOLevelState::lives`
is signed, `spend_lives_on_death` decrements it off `ActorDiedMessage`, and all
three death roads reach it (the hit resolver, `publish_kernel_reset_death` for a
pit or hazard, `publish_timeout_death` for the clock). It landed in `52e34be60`
with coverage at three scopes. ⭐ **I wrote the row from the `▢` in Jon's
observations file without grepping HEAD** — a marker in a maintainer's file is a
REPORT, not a measurement, and his file is explicitly one he edits at his own
pace.

⇒ what was actually wrong was **three doc claims that outlived the behaviour they
described**: the field's own first line, the `spend_lives_on_death` block saying
*"At zero lives the RUN is over: lives, score and clock return to their starting
values"*, and a test NAMED `..._and_zero_lives_restarts_the_run` whose body
asserts `lives == -1`. The code carried a ⛔ *"no floor and no reset"* note beside
three sentences promising the opposite — i.e. promising the game-over that Jon's
"for now" clause explicitly ruled out. Corrected in place; no behaviour touched.

⇒ **the FIRE FLOWER question now has a DECISION PROCEDURE, landed 2026-08-14** —
`a_grown_mary_o_bonks_a_question_block_and_wears_the_fire_flower`
(`level_1_acceptance.rs`, DEFAULT feature set: `cargo test -p ambition_demo_mary_o_app`,
36 → 37; the module is `#![cfg(not(feature = "input"))]` so `--features visible`
compiles it out). ⭐ **every pre-measurement beat fails with a `HARNESS:` prefix,
so an inconclusive run cannot be misread as "the level cannot".**

⇒ **and the reading of Jon's report has sharpened.** 1-1 authors three ladder
blocks, all defaulting to `Toward(Lantern)`, and a grown bonk pays the beacon. Two
substantive reasons it FEELS unavailable even if the test passes: **the first
`?`-block you meet can never pay it** — a small Mary-O is always paid the wand, so
the beacon needs a SECOND block, and 1-1's second is past pit A and behind a warp
pipe — and **the beacon is `ItemMotionPlan::still()`**, so it waits on top of the
block instead of walking to you like the wand.

⭐ **"you cannot bonk it from underneath standing still" is a jump-TECHNIQUE fact,
not Jon's bug — but the adjacent fact IS the bug's shape.** From her authored law
a standing jump raises her centre 144.8px and the underside sits 96px up, so
bonking from beneath is comfortable. What a standing jump cannot do is get her ON
TOP: the top face is 128px up, leaving 17px of margin and ~0.25s of hang, and
`air_coast_decel: 0` means air acceleration alone crosses only ~12px horizontally.
⇒ that is why `mount` (written for a 64px pipe rise) could never reach a `?`-block
and why the earlier attempt stalled; the new `hop_onto` leaves the ground MOVING
and cycles run-up length and side.

⚠ **a latent fixture trap found beside it:** the existing `first_power_block()`
matches by LOOK, and 1-1 has two `AlwaysQuasar` blocks wearing the same look — so
it names x=192 only by converter emission order. The new helper asks CONTENTS
instead. The old one is used only by an `#[ignore]`d run and was left alone.

⛔ **THE MARY-O PRESENTATION GUARDS DO NOT RUN IN THE ORDINARY SUITE**, and that
is the thing to fix before chasing another "it looks wrong" report. The
`painted_blocks_still_change_their_art` module is `#![cfg(feature = "visible")]`:
`cargo test -p ambition_demo_mary_o_app` runs 36 tests, `--features visible`
runs 44. The eight it adds are the ones that assert what a block LOOKS like.

⇒ measured 2026-08-14: the maintainer row *"a discovered hidden block still pays
out invisibly"* is **stale** — `a_discovered_hidden_block_reveals_itself` passes
at HEAD, asserting the struck block wears `SpentBlockTile`. It simply never ran,
so nothing said so and nothing would have caught a regression either. Any
Mary-O visual work should run the `visible` suite before and after; it takes a
~5 minute cold build and is the only thing watching that surface.

### The hole is not Mary-O's, it is the whole workspace's — measured 2026-08-14

`scripts/feature_gated_tests.py` already answers this and nobody had run it
against a decision: **24 crates hide 629 tests behind features.** Eight of them
were run explicitly at HEAD and every one is GREEN, so this is a prospective
hole, not a live bug:

| crate | bare | with its features |
|---|---|---|
| `ambition_demo_mary_o_app` | 31 | 45 (`visible`, 9.7s) |
| `ambition_demo_sanic_app` | 25 | 45 (`visible`, 4.9s) |
| `ambition_touch_input` | 4 | 45 (`mobile_touch`) |
| `ambition_audio` | 25 | 64 |
| `ambition_portal2d_presentation` | 16 | 45 (`effect_view_cones`) |
| `ambition_input` | 54 | 115 (`input`) |
| `ambition_game_shell` | 45 | 70 (`basic_presentation`) |
| `ambition_dialog` | 30 | 42 |

⛔ **and there is no automatic runner for any of it.** `.github/workflows/test.yml`
is `on: workflow_dispatch` only — disabled 2026-05-07 by Jon (*"no need to churn
the servers with rust CIs until we have something we really need github action
testing for"*) — and `scripts/gate_suite.py`, the per-turn suite, runs exactly
`cargo test -p ambition_app --test app_it`, deliberately shrunk on Jon's own
measurement (*"I want to bias towards running less tests"*).

⇒ **so what runs is a maintainer decision, not an agent's**, and it is filed in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) with the
prices above rather than answered by quietly enlarging the gate he shrank. ⛔ do
not add these to `gate_suite.py` without that answer; ⛔ do not add a CI job to a
workflow that does not run and call the hole closed.

---

## Waiting on an external fact or maintainer decision

These are real unresolved items but are deliberately **not** `▢` queue work.

- **D23 — projectile collision feel:** authored hurt geometry versus coarse body
  box; see [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).
- **D50 — dropped held-item lifetime:** room-scoped versus persistent-world
  semantics; see the decision inbox.
- **D53 — Android suspend/resume:** validate the residual behavior on a real
  device before opening another source fix.
- **D54 — reported visual/VFX issue:** needs the requested reproduction.
- **D70 — Mary-O restart observation:** current tested paths do not reproduce it;
  needs game/room/time context.
- **D42 / D47 — character sizing/rig art:** currently principally authored
  rig/body-inset and visual-review work unless a reproduced engine defect appears.
- **D114 — fighter-vs-fighter hit emphasis:** product feel for non-primary-seat
  combat; see the decision inbox.

---

## Regressions repaired 2026-08-14 (diagnosed by Jon, fixed with falsifiers)

| | status |
|---|---|
| Shield+Attack held-item throw | ✔ **closed**, confirmed by Jon on retest |
| F1 geometry coherence | ◐ **fix + regression coverage landed (D116 M2a); AWAITING JON'S VISUAL RETEST** — every rigidly attached row now takes one `PresentedPose::delta()`, and a test pins that. ⛔ an automated test is not a picture: the FIRST fix was also green, and it had merely moved one attached family to the presentation clock and relocated the shudder |
| Smirking eye beam | ✔ **closed** — never a replay fault; a contact predicate measuring centre distance, so its severity scaled with body size |
| Smirking same-room replay | ◐ **behaviour restored, class still open** — persistence ordering and one constructor divergence are fixed; same-room replay is still a second constructor |

⭐ **the recurring SHAPES moved to `dev/benchmark-candidates/`**, which is where
AGENTS.md routes durable lessons; this ledger keeps status, not case files. Two
of the four generalised:

- [`two-constructors-for-one-population`](../../dev/benchmark-candidates/two-constructors-for-one-population-2026-08-14.md)
  — a constructor is TOLD a fact, a reset RE-DERIVES it from a proxy, and *"leaving
  the room and re-entering fixes it"* is the tell handed to you for free.
- [`an-absent-component-reads-as-no-value`](../../dev/benchmark-candidates/an-absent-component-reads-as-no-value-2026-08-14.md)
  — a narrow population's `None` arms silently serve "not covered", so widening
  it fixes three consumers and makes a fourth live and wrong.

The remaining two are already invariants elsewhere: an action spends an input
edge where it COMMITS (portal adapters), and a bare set tuple is not a sequence
(`ContentDialogueFollowupSet → ContentRoomReplayResetSet → RoomReplayApplied`
needed `.chain()`; the comment above it already claimed the order).

⛔ open item, carried forward: same-room replay converges on canonical
reconstruction, sequenced after the instance lifetime/provenance model — see
[`engine/same-room-replay-is-a-second-constructor.md`](engine/same-room-replay-is-a-second-constructor.md).

- ▢ **D127 — Deterministic authored gameplay logic and orchestration. M0 COMPLETE; M1 MET FOR CONDITIONS — provider AND consumer sides — and OPEN FOR COMMANDS.**

⛔⛔ **THIS ROW SAID "M1 PARKED" FOR A DAY AFTER M1'S ACCEPTANCE WAS MET.**
Corrected 2026-08-16 against HEAD, not against the plan's prose. M1 asks for two
domains exposing semantic vocabulary through a domain-owned contract, with the
behavioural test that adding the second provider edits **no central enum**. All
three clauses hold: `shared_tangle::authored_logic` owns the contract with a
PRIVATE `publish` reachable only through `PublishCondition for App`;
`items/pickup` publishes `custody.is_held` and `world_facts` publishes
`world.flag_set` from unrelated domains; `world/gated_lock_walls` consumes one
and names no flag; and `INTRO_FLAG_GATED_LOCK_WALLS` plus its 136-line const
table were deleted in the same slice.

⭐⭐ **AND THE RIVAL MECHANISM IS GONE (2026-08-16).** Authored `.yarn` had its
own way to ask the world things — hand-written Yarn library functions over a
per-frame `YarnStateMirror` — so a lock wall asking `world.flag_set` and a
dialogue asking `flag("...")` were one question through two unrelated
mechanisms. One generic verb, `condition("domain.question", <arg>)`, now
forwards authored dialogue to whichever domain published the answer, with **no
edit to any bridge or vocabulary table** to add a question. `inventory.holds`
was published as the third provider in one line of composition; `flag(id)` and
`inventory_has(item)` were **deleted** along with both mirror slices, the
per-frame inventory refill, a duplicate item-spelling normaliser and a
zero-adopter `legacy_dialog_alias`.

⛔ **the belief that forced the mirror was FALSE at HEAD.** Three module headers
said Yarn functions cannot be Bevy systems; `bevy_yarnspinner` runs the
interpreter from an exclusive system and threads its `&mut World` down to
`YarnFn::call_with_world`, and `SystemId<In<P>, O>` implements `YarnFn`. The
verb reads the live world. ⚠ two honest limits recorded in the plan: Yarn's VM
asserts exact arity so the verb takes exactly one argument, and
`ParamKind::Reference` is refused rather than coerced from a quoted string —
that refusal is M2's to replace.

✔✔ **AND THE CONDITION HALF NOW HAS A SECOND, VERY DIFFERENT CONSUMER
(`61b4cd836`, 2026-08-16): AUTHORED DIALOGUE.** One verb —
`condition("domain.question", <arg>)` — names no question, no domain and no flag;
it forwards whatever a `.yarn` line asks to whichever domain published the answer.
**Publishing a condition makes it askable from authored dialogue with no edit to
a bridge or a vocabulary table**, which is the same behavioural acceptance the
second provider had. `inventory.holds` was added as a third provider in ONE line
of composition.

⛔⛔ **AND IT REFUTED A PREMISE THAT THREE MODULE HEADERS ASSERTED.** They all
said Yarn library functions cannot be Bevy systems and so cannot reach `&World` —
which is why `YarnStateMirror` existed at all. Measured at HEAD it is **false**:
`bevy_yarnspinner` advances the interpreter from `continue_runtime`, an
**exclusive system**, and threads `&mut World` down through
`Dialogue::continue_with_world` to `YarnFn::call_with_world`, and
`SystemId<In<P>, O>` implements `YarnFn`. ⇒ a Yarn function IS a Bevy system and
DOES get the live world. **A design constraint recorded in three places was never
re-measured.**

⭐ **deletions, which is what makes it a slice rather than an addition**: `flag(id)`,
`inventory_has(item)`, both mirror slices, the per-frame inventory refill with its
legacy-alias table, `normalize_item_id` (a second copy of `Item::from_dialog_id`'s
rule), `mirror_inventory_has`, `Item::legacy_dialog_alias` (zero adopters once the
refill went) and three tests that pinned them. What survives of the mirror is
documented as a **projection**: boss/quest state, visit counts, wallet and content
`extras` have no published condition, and two of those return `f32`, which a
boolean verb cannot express.

⚠ **two limits recorded rather than hidden.** Yarn's VM asserts a call's argument
count equals the registered function's parameter count and counts `Option`
parameters, so a variadic bridge is **not expressible** — the verb takes exactly
one argument, which every published condition wants. And a `ParamKind::Reference`
argument is **REFUSED with a reason** rather than coerced from a quoted string:
⭐ *a `.yarn` literal is not an identity, and answering confidently about
whichever occurrence happens to share the spelling is worse than not answering.*
The refusal is something M2 can replace; a wrong answer is something M2 would
have to find first.

⚠ **THE STILL-MISSING HALF: COMMANDS HAVE NO PROVIDER CONTRACT.** Conclusive grep — no `PublishCommand`, no command
catalog, nothing. ⭐ **that half is not "one more of the same"**: a condition is a
*question about the world* and is safe precisely because it cannot change
anything, while a command mutates and therefore owes **authority** (who may run
it), **ordering** (when in the frame) and **rollback semantics**. ⇒ M4's warning
— *rollback is a design input, not a cleanup afterwards* — lands on the command
half directly, and M2 can only ever prepare a PREDICATE until the command
vocabulary exists.

⭐⭐ **THE CONDITION SIDE'S LOAD-BEARING TRICK, MEASURED 2026-08-17 — the command
half has to reproduce it or it cannot be waived from rollback.**
`ConditionCatalog::publish` is **PRIVATE**; the only way in is the
`PublishCondition` trait on `App`; and **a simulation tick holds a `World`, never
an `App`**. So *"immutable once the simulation starts"* is a property of the
TYPE, not a promise in a comment — and that is precisely what earns the catalog
its rollback waiver. Its own doc says making `publish` public *"for convenience
would silently convert the waiver into a lie."*

⇒ **so the command catalog's FIRST design constraint is not authority or
ordering — it is being unmutable at runtime by construction**, exactly this way.
A command registry a system could write to is rollback state, and then every
authored verb is in the snapshot.

⛔⛔ **BUT M1-FOR-COMMANDS HAS NO CUSTOMER YET, AND THAT IS THE ACTUAL BLOCKER —
counted 2026-08-17.** The condition half's full ecosystem, publishers separated
from consumers:

```text
PUBLISH (a domain answering a question)   world_facts · items/conditions ·
                                          items/pickup    →  3 questions
CONSUME (something asking one)            dialog/authored_conditions.rs
                                          world/gated_lock_walls.rs   →  2
```

⭐ **two independent consumers is a real ecosystem, not a demo** — so the
contract earned its place twice: once by the deletion above, once by adoption.

⭐⭐ **and yet neither consumer wants a command, which is why `PublishCommand`
does not exist.** Both ask a question and then perform an effect **they own
intrinsically** — a wall opens; a dialogue line is offered. The effect is not a
choice an author made, so there is nothing for a command vocabulary to carry.

⇒ **a command catalog is needed exactly when the AUTHOR picks the effect** —
which is what this row's own examples are: *"when two switches are active, power
a lift"*, *"when an item is placed here, open a gate"*. Today **no authored
surface lets an author choose an effect at all**, so building the catalog first
would be vocabulary with zero speakers — the precise thing
`authored_logic`'s header forbids, and the standard it holds ITSELF to.

⭐⭐ **THE CUSTOMER IS FOUND, 2026-08-17 — and it is the same SHAPE the condition
half deleted to earn its place.** `game/ambition_content/src/encounters.rs`:

```rust
const KERNEL_FACES: [(&str, &str); 4] = [
    ("kernel_switch_down",  "gravity_down"),
    ("kernel_switch_left",  "gravity_left"),
    ("kernel_switch_up",    "gravity_up"),
    ("kernel_switch_right", "gravity_right"),
];
```

⇒ **a hand-kept const table pairing AUTHORED ids to hardcoded behaviour, read by
a bespoke system** — which is the condition half's deletion gate
(`INTRO_FLAG_GATED_LOCK_WALLS`) word for word. This row's headline example is
*"when two switches are active, power a lift"*; the symmetry room is *"when
these four switches are visited, complete an attunement"*.

⭐⭐ **and the asymmetry is EXACT, which is what makes it the first command
rather than a candidate** — the engine publishes the QUESTION and cannot express
the ANSWER:

```text
world.flag_set(<flag>)   ✔ published (world_facts.rs) and ADOPTED
world.set_flag(<flag>)   ⛔ absent — SYMMETRY_ATTUNEMENT_FLAG is set by bespoke Rust
```

⇒ **`world.set_flag` is the first command.** Its condition twin already ships and
has consumers, and the deletion it buys is a const table plus the reducer reading
it. ⚠ it also answers the rollback question cheaply: a save flag is already
snapshot state, so this command mutates something the sweep covers rather than
introducing a new kind of write.


▢ **so M1-for-commands is BLOCKED ON A CUSTOMER, not on design.** The first
question is not *what shape is a command* but *which authored thing gets to name
its own effect first* — pick that, and the three below stop being hypothetical.
⭐ the rule that caught this is *count the ADOPTERS, not the capability* — a
shipped capability can have zero.

▢ **and then the three the row already names, in the order they bite:**
1. **rollback semantics** — M4's *"rollback is a design input, not a cleanup
   afterwards"* lands here. A command that mutates during a predicted frame must
   either be rewound or be provably idempotent; decide which BEFORE the
   vocabulary, because it decides the vocabulary.
2. **ordering** — a condition is safe at any point in the frame because it reads;
   a command has a phase. Name it as a SET, below the monolith, or the first
   carve that moves an authored domain re-pins it (D33 step 1.5's lesson).
3. **authority** — who may run it. ⚠ note the condition side got this free by
   being read-only, so there is no precedent to copy here; it is genuinely new.

⚠ **duplicate-id panics at startup, by design** — *"the alternative is that the
winner is whichever plugin happened to build last, which is a bug that only
appears when a host changes its plugin order."* Commands owe the same.

Plan: [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md)
(new, 2026-08-15). Maintainer-identified capability gap: **authoring is strong for
nouns — characters, items, rooms, encounters, sprites, music, platforms, portals,
capabilities — and weak for verbs and relationships over time.** *"When two
switches are active, power a lift"*, *"when an item is placed here, open a gate"*,
*"latch this once true"*, *"wait for a semantic event, then act"* all currently
fall through into bespoke Rust.

The doctrine, recorded and settled: **Rust extends the engine's vocabulary;
authored gameplay content composes vocabulary that already exists.** The
deterministic simulation still determines what is true — authored rules invoke
explicit semantic domain operations and never mutate arbitrary ECS state.

⛔ **this row does NOT demote D125 or the first capability-aware reachability
customer.** It is deliberately behind both. D125 is what makes a condition like
*"item occurrence X is held by body Y"* answerable at all, so it is a reason to
finish D125, not to pause it.

⛔ **NOT authorized:** a `UniversalRuleVM`, Lua/Rhai, arbitrary ECS reflection, a
universal scene graph, a central `EngineEffect` enum, or replacing any existing
encounter/cutscene/boss/moveset representation. The several partial
condition → effect systems already in tree are **evidence and candidate
customers**, not defects.

✔ **M0 is COMPLETE** (14 systems inspected at HEAD, 2026-08-15). ⛔ **and M1's
condition half is complete too**, provider contract and consumer alike — see
above. The executable step is **M1-commands or M2**, and they are not
independent.

⇒ **M0 RAN 2026-08-15 and narrowed the design. 14 systems inspected.** Findings,
in order of consequence:

1. ⛔ **the shared substrate does not own a universal SEQUENCER, and the existing
   domain sequencers are not to be forcibly unified.** A monotonic cursor
   (`EncounterScript`), a reversible cycling timer machine (`tick_gate_portal_phase`)
   and a subroutine stack with interrupts and seeded selection (`BossPatternState`)
   are three execution machines; one shared form covering all three needs a branch
   naming its customer. ⇒ **the substrate is conditions + commands + prepared
   references + preparation + discovery.** ⚠ that is the proven statement — it is
   narrower than *"sequencing can never be shared"*, which is not proven and is
   not claimed. A reusable control-flow backend (Bonsai or otherwise) stays an
   **optional later experiment** if several genuine customers would benefit; it is
   not a current priority.
2. ⭐ **the gap is on the CONDITION side.** There is no shared condition/predicate
   type anywhere in the workspace. The effect side already has 5+ typed command
   buses — and a monolithic `GameplayEffect` enum was **built and already
   deleted**. The "no god enum" non-goal is a repeated experiment, not a taste.
3. ⛔ **boss patterns are the TEMPLATE, not a customer** — my first draft had this
   wrong. They already ship authored `.ron`, a content-pack schema family,
   compile-time cross-ref resolution, a design validator, and a cursor that
   snapshots the **resolved** timeline. Copy them; do not migrate them.
4. ⛔ **M4 is not deferrable.** Three shipped answers exist to *"is a program
   counter rollback state?"* — register cursor+program (cutscene, `MovePlayback`),
   register nothing and rebuild (`EncounterScript`), or waive it (gate portal).
   Whichever the shared form picks changes ≥2 shipped systems.
5. **customers chosen:** A = the cut-rope Smirking Behemoth (deletes
   `setup_cut_rope_encounter`, five consts, and the *"despawn so the script
   rebuilds itself"* reset arm that exists only because the script is Rust-built);
   B = intro flag chains + flag-gated lock walls (deletes two const tables, two
   systems, `IntroLockWallCache` **and its invalidation rule**, and the
   `Update`-instead-of-sim-schedule defect by construction). ⚠ B's honest
   weakness: 4 of its 5 chain targets are dead vocabulary — **the live deletion is
   the lock-wall half**, and the campaign must not count dead rows as value.
   ⛔ **moving-platform gating was REJECTED** — the plan's own headline example,
   and it has nothing to delete. Pure addition is falsifier 2.
6. **no `bonsai-bt` and no behavior-tree/state-machine crate of any kind is in the
   lockfile.** A BT backend would be a new dependency solving *sequencing* — the
   one part finding 1 says stays domain-owned. Not refuted, no longer near front.

⇒ **the two defects the census flagged, both resolved — details live in the code
and the plan, not here:**

- ✔✔ **the gate portal's rollback waiver was a REAL DESYNC, fixed 2026-08-15.**
  ⭐ **the input rewound and its integral did not**: the switch is
  rollback-registered, the phase is that switch integrated over time and was not,
  so a rewind left the integrator permanently ahead — and the consumer *refuses a
  room crossing*, so it is authoritative, not cosmetic. The waiver was a **true
  sentence about the wrong half of a value**. Fix deleted `GatePortalConfig.phase`
  and registered a `GatePortalPhases` resource **with a value projection**, because
  a presence-only probe would have passed while reproducing the defect. Schema
  **31**; baseline and absence contracts re-run and green. ⇒ full rationale at the
  registration site in `rollback/mod.rs`; ⭐ the generalisable shape is *"a
  registered input with an unregistered integral"*.
- ✔ **the `Brain` cursor's `_` arm — MOSTLY DISSOLVED.** Six brain families looked
  like they failed to rewind; `rollback_component_cursor` clone-snapshots the whole
  component, so they rewind fine and what they lack is **desync DETECTION**. Real
  but much smaller than reported.

⇒ related, and deliberately NOT its own campaign: **A9** in
[`engine/authoring-and-tools.md`](engine/authoring-and-tools.md) — the
project-wide semantic dependency/reference graph (reverse references, structured
unresolved-reference diagnostics, transactional rename planning). It is an
extension of existing authoring/inspection tooling.

⇒ doctrine correction landed with this row, because it was being misread: **a
central *authoritative* census that every new domain must edit is bad; a derived
*read-only* discovery index that domains contribute descriptors to is good and is
required.** Recorded in `simulation-authority-and-determinism.md` (at the goal
statement that gets cited as the objection) and in
`inspection-diagnostics-and-workbench.md` (which owns discovery). ⛔⛔ do not
sacrifice discoverability in the name of avoiding central authority.

- ✔ **D133 — THE DURABLE SAVE HORIZON. What the world remembers about occurrences
  now survives closing the program. (opened and LANDED 2026-08-16)**

⭐⭐ **THE RESULT IN ONE SENTENCE: the on-disk form IS the checkpoint's own
description, serialized — not a fourth description of the same facts.**
`AmbitionGameSaveData` gained three `#[serde(default)]` lists that are
`AuthoredOccurrences`, `CustodyBaseline` and `MintedItemBaseline` field for field
(`CURRENT_SAVE_VERSION` 3 → 4). ⭐ **that the file needed no field the checkpoint
slice had not already measured is the finding**: `identity + provenance +
definition-REFERENCE` was derived from what a checkpoint owes a hand, and a save
asks the same question and gets the same answer.

⭐⭐ **AND A LOAD IS A CHECKPOINT RESUME.** The loader adopts the ledger, adopts
the three baselines from the same file, and writes one `ResetToCheckpoint` —
after which the road a death already takes rebuilds the world. ⇒ **two systems and
no new reconstruction logic**, and there is still exactly one authority on what a
room owes the world.

| Falsifier | Verdict | Proof |
|---|---|---|
| A — an authored object carried to another room and dropped is lying there after a load, same `SimId`, home pedestal EMPTY | ✔ | `an_object_left_in_another_room_is_lying_there_after_a_load` — one file, loaded into BOTH rooms, plus a default-file control run |
| B — a held weapon is still in the same hand after a load (D132's LOSS, closed) | ✔ | `a_weapon_in_your_hands_is_still_in_your_hands_after_a_load` — both halves of the forked relation, plus an empty-handed control |
| C — a terminal row is not undone, and an untouched record is untouched | ✔ | `a_consumed_occurrence_is_not_resurrected_by_a_load_and_an_untouched_one_is_untouched` — the subject has NO live entity, so only the wrong implementation can act |

⛔⛔ **A DEFECT THE FIXTURE FOUND THAT NOTHING ELSE COULD HAVE.** A session builds
its start room before any file is read, so the instant the loaded ledger arrives
the world holds an occurrence the file says is elsewhere — and
`record_placed_ground_items` republished the stale position over the loaded row,
sending the object home and resurrecting a terminal row. ⭐ **the fix is an
INVARIANT**: an occurrence comes to rest here only if its row says `InCustody` or
already says `Placed` here, because **an object cannot change rooms without being
carried**. It refuses rather than repairs, so it is not a second reconstruction
authority.

⚠ **`GGRS_ROLLBACK_SCHEMA_VERSION` 33 → 34 for a RENAME and nothing else**:
`resource.inventory_restored` → `resource.save_restored`, because the latch now
means "the loaded save has been applied" and the occurrence leg reads it too.
⭐⭐ **the save file is not rollback state** — the three values it serializes were
already registered at v32/v33, which is the same sentence as "horizon 3 is a
serialization of horizon 2". Also fixed: the durable leg was installed by the
visible-binary-only presentation assembly, so **no headless composition saved or
loaded anything**; `DurableSaveHorizonPlugin` owns it now.

⚠ **STILL OPEN after this**: a runtime mint NOT in a hand at save time (lying in a
room, in flight) is undescribed and lost — the description remembers no position;
`Consumed` round-trips through the file and still has no live PRODUCER;
`load_save_at_startup` is still presentation-only, so a headless composition
mirrors into `AmbitionGameSave` and never writes a file; and the body resumes at
the shrine while the objects resume at the autosave's instant.

- ▣ **D132 — THE SAME ITEM HAS TWO PERSISTENCE AUTHORITIES AND THEY HAVE NEVER
  BEEN ASKED TO AGREE. (opened 2026-08-16; MEASURED and HALF CLOSED 2026-08-16)**

⚠⚠ ~~**A PLAYER-VISIBLE TRADE WENT WITH THIS SLICE: A HELD WEAPON CARRIED ACROSS
A SAVE/LOAD IS NOW LOST RATHER THAN DUPLICATED.**~~ ✔✔ **THE LOSS IS CLOSED
(D133, 2026-08-16).** The durable save describes custody now — as an OCCURRENCE
(identity + whereabouts + the hand), never as a quantity — so the weapon comes
back in the same hand, and the two populations stay disjoint so the duplication
does not return by the new road.
`a_weapon_in_your_hands_is_still_in_your_hands_after_a_load` is the proof, with a
default-file control run so neither claim can pass by accident.

⭐⭐ **MEASURED FIRST, and the prediction below was wrong about which history
breaks.** `two_persistence_authorities_for_one_item.rs` drives the exact scenario
this row asks for — save a count of 1, load, equip out of the count table, throw
(which mints), pick up, bank at a shrine, die — and answers: the player ends up
**holding it AND owning it**; the count is decremented **never, at any beat**;
and the second save round-trip **agrees with the hand**, by coincidence rather
than by rule. That history is not the defect.

⛔ **THE DEFECT IS NEXT DOOR: `OwnedItems` is not checkpoint state at all.** The
pressed pickup used to `grant` a catalog row beside taking custody, so ONE
acquisition left TWO records and only the object's rewound. Acquire a weapon
after the checkpoint, die, and the object goes back on its pedestal while the row
stays — the menu then equips the phantom and the throw mints a SECOND real
weapon, and the durable save writes the phantom to disk on the way past.

⭐ **CLOSED BY DELETION, both halves probe-falsified.** The `grant` at the pickup
is gone (the object is the record), and `OwnedItems::count` PROJECTS the equipped
slot so the grid shows what the hand holds and loses it exactly when the hand
does. `to_persisted` reads the stored quantity, never the projection. Restoring
either half turns the fixture red. No schema change — no field moved.

⚠ **STILL OPEN, and the gate is named**: a quantity conferred by
`<<give_item>>`/shop/drop keeps its row through the mint, so it can still
manifest a second object. Spending the row at the mint annihilates it on a death
that retracts a post-checkpoint mint. ⇒ `OwnedItems` must join the checkpoint
baseline first, and the mint spends the row in that same change.
`a_granted_quantity_survives_the_death_that_retracts_the_instance_minted_from_it`
is the poison against retracting the row at the reset instead.

⚠ ~~**and the durable-save leg runs in NO headless composition**~~ ✔ **FIXED
(D133, 2026-08-16).** `DurableSaveHorizonPlugin` in the runtime plugin group owns
the latch and all four persist systems; the fixture steps a frame now instead of
calling the shipped functions by hand.

⇣ the original statement of the row, kept because its second half is the gate:

⭐ **the durable-save frontier's first real problem, and it is not "write a file".**
Two mechanisms already persist the player's possessions, by different keys, on
different clocks, with no relationship between them:

```text
DURABLE SAVE      items/persist.rs mirrors OwnedItems (the 24-slot catalog) and
(disk, on load)   BodyWallet into AmbitionGameSave, keyed by stable `dialog_id`
                  -> restores a COUNT

CHECKPOINT        MintedItemBaseline + CustodyBaseline capture at a shrine rest
(memory, on death)-> restores an INSTANCE into a specific hand, keyed by SimId
```

⛔⛔ **and the same physical object crosses between them, by design.** The
inventory menu equips straight out of the count table; throwing what it equipped
mints an instance (`SimId::spawned`) — that is the production road D125's last
slice was built on. So one gun sword can be a row in `OwnedItems` at the same
time as an occurrence in the custody baseline.

⇒ the failure this predicts, and it should be **written as a fixture before it is
designed**: save with a count of 1, load, mint an instance of that same spec,
bank it at a checkpoint, die. Does the player end up holding it **and** owning
it? Is the count decremented once, twice, or never? ⚠ nobody knows, because no
test has ever put both authorities in the same sentence.

⭐ **this is the "one class wide" seam already measured on D125**: 5 of 6 catalog
classes are counts forever and their readers legitimately want a quantity; the
whole problem is the **nine held weapons/abilities that are an instance and a
count at once**. ⛔ so the answer is NOT to give the count table a row per
object — that was already rejected. It is to decide which authority owns those
nine, and to make the other one *derive* rather than *store*.

⚠ related and unclosed in the same area: a minted instance **not in a hand** at
the commit — lying in a room, or in flight — is still undescribed and still lost.
That is exactly where *"a hand needs less than a world"* stops paying: it needs a
position, and position is the first thing the description would grow.

- ✔ **D134 — CLOSED 2026-08-16. The suite is 34/34, zero violations; the gating
  call is Jon's and is costed in `awaiting-maintainer-decision.md` item 13.**

⭐ **the twelve were four different things wearing one label**, and the count was
the least informative fact about them:

```text
1  hazard, not defect   gate_portal `phases` HashMap -> BTreeMap. The flagged
                        site collect()s and immediately sort_by()s, so the hash
                        order NEVER reached an observable. Fixed anyway, because
                        the ordering should be the type's property rather than
                        the next editor's discipline.
2  REAL ADR 0024 §1     `Option<&MotionModel>` — and it was hiding a SECOND one
                        the rule could not spell (`Option<&ae::MotionModel>` in
                        `perception_body_for`, whose `None` arm read a missing
                        component as `AxisSweptMotion::default()`).
2  policy imprecision   a PRE-SPAWN `ActorClusterSeed` and an off-sim
                        `BodyClusterScratch`. Neither is an entity; neither has a
                        frame or an integrator, so there is no authority to route
                        through. Both now state their initial state at
                        construction (`place_at`, `with_velocity`).
1  contract OUTLIVED    `player-fallback-update-documented` demanded a review
   its subject          marker for a slot-ordered fallback `333c48376` deleted.
                        DELETED, with a tombstone; its sibling on `save_sync.rs`
                        still guards the one surviving fallback.
7  the POLICY was       runtime -> ldtk. See below.
   wrong
```

⭐⭐ **THE `runtime → ldtk` CLUSTER WAS NOT AN UPWARD DEPENDENCY, AND `cargo tree`
SETTLES IT IN ONE LINE.** `ambition_platformer2d_ldtk`'s entire transitive closure
contains **zero** occurrences of `ambition_platformer2d_runtime` or
`ambition_platformer2d_actor_monolith`; its own dependencies are
`asset_manager`, `entity_catalog`, `platformer2d_core`, `shared_tangle` and
`platformer2d_world` — the same graph depth as `ambition_platformer2d_world`,
which the runtime's allowlist has permitted all along. And the monolith, also
allowed, has linked it directly the whole time. ⇒ **the edge is downward**, and
`runtime-manifest-deny` + `runtime-source-no-upper` were stating one wrong fact
twice. Both changed, with the argument written into their own `rationale` fields.
⚠ `bevy_ecs_ldtk` stays denied in both: the runtime may compose the adapter, never
the backend the adapter exists to contain — poison-tested red to prove that half
still has teeth.

⚠ **and the story is on its THIRD instance in the same file.** The allowlist
already carries two comments saying a laundered edge became a declared one when a
facade was deleted (`ambition_sprite_sheet`, `ambition_character_sprites`); this is
the same event for the monolith's `ldtk_world` facade, deleted 2026-08-15. ⇒ the
lesson is not about LDtk: **deleting a compatibility facade reddens a boundary
policy, every time, and it is the second file nobody remembers to edit.**

⚠ **the concern the deny row was groping at is real and is now D135**, separated
from the manifest edge it could not express.

⚠ **one blind spot left standing, deliberately and named**: the movement rule
matches SPELLINGS, and `Option<&ae::MotionModel>` — `ae::` being the alias most of
the workspace imports engine-core under — escapes it. One production site still
uses it (`ambition_sim_view/src/combat_geometry_view.rs:152`, an observation
read-model whose whole body-cluster group is optional). Widening the rule tonight
would have reddened a crate this slice did not analyse; it is recorded in that
policy's rationale.

- ✔ **D137 — CLOSED 2026-08-17. The doc-link ratchet was RED, is now GREEN, and
  is now RUN BY CI. (opened 2026-08-16)**

⭐ **the second half is wired**: a `--check` step at the end of
`sandbox-headless-smoke`, which already installs Bevy's system dependencies,
already builds the monolith and already has a warm cache — the ratchet runs
`cargo doc` per crate, so the pure-Python `agent-kb-check` job could not host it.
Last in the job, so a doc regression cannot mask a test failure.

⛔⛔ **AND THE FLAG IS THE WHOLE GATE — `--check` IS LOAD-BEARING.** Measured
by poisoning the baseline (`ambition_combat` 13 → 5) and running both ways:

```text
python3 scripts/check_doc_link_ratchet.py            ⛔ prints "ROSE from 5", exit 0
python3 scripts/check_doc_link_ratchet.py --check    ⛔ prints "ROSE from 5", exit 1
```

⭐ the enforcing arm is `if risen and args.check`; the bare command is
**advisory by design**. A step wired with the obvious invocation would have been
a gate that CANNOT FAIL — which is worse than no gate, because it reads as
coverage. ⚠ this is why the local-gate measurement (338 s) answered NO and CI
answered YES: the cost was never the only question.

```text
ambition_platformer2d_actor_monolith   131  ⛔ ROSE from 122
ambition_characters                     51  ⛔ ROSE from 39
```

⭐ **RE-MEASURED 2026-08-17: still red, and it had crept further.**
`ambition_characters` had reached **53** — this campaign added two more while
renaming the smash brain's closing verb. Both repaired (`ed6bf307c`), so the
crate is back to the 51 above:
* a link through `movement::abilities::…`, which is a PRIVATE module — the
  function is published by a `pub use` one level up, and the doc named the
  definition rather than the address.
* a bare `StateMachineCfg::Smash`, which is a REAL variant that simply is not in
  scope where that doc sits. ⚠ worth knowing, because it reads as a deletion and
  is not one: the ratchet cannot tell "this no longer exists" from "you did not
  say where it lives", and only the first is rot.
⚠ the remaining 51 vs 39 (and 131 vs 122) is the debt the row was opened for and
is NOT repaired. Fixing what we broke is not the same as paying it down.

⛔ **`python3 scripts/check_doc_link_ratchet.py --check` fails**, and like the
workspace policy suite before it (D134), **nothing per-turn runs it** — the gate
is `cargo check -p ambition_app --all-targets` + the app suite + Smash.

✔✔ **THE RATCHET IS GREEN AS OF 2026-08-17 (`f7e34225d`)** — `ambition_characters`
is back to its 39 baseline, so no crate has risen and `--check` exits 0 for the
first time since this row was opened.

⛔⛔ **AND THE LEDGER WAS ABOUT TO BE HANDED A 13-LINK IMPROVEMENT NOBODY
EARNED.** The conversation carve (D33 step 2) took the monolith 122 → 109, and
the script invited banking it. Those thirteen were not FIXED, they were RE-HOMED
into a crate `CRATES` did not name. `ambition_conversation` is on the list now,
which puts **11 still-broken links back on the books** and moves the honest total
UP: 182 → **193**. ⇒ **a carve launders debt off any ledger keyed by crate name,
and the ledger congratulates you for it** — the rule ("the destination joins in
the same commit") is written into `CRATES` beside the entry.

⛔⛔ **AND THE ROW'S REMAINING ASK — "put it in a gate" — IS ANSWERED NO, BY
MEASUREMENT.**

```text
warm, nothing changed                     51 s
after touching ONE crate                 338 s   (5.6 min)
```

`ambition_platformer2d_core` is upstream of the other four, so touching it
invalidates all five doc builds — and a gate runs precisely AFTER edits, so 338 s
is the honest number, not 51. Against a project gate that is already
`cargo check -p ambition_app --all-targets` + a ~150 s app suite, this is not a
per-turn check. ⭐ **the row's own prose predicted it** — *"a ratchet over the
whole workspace is a slow check that nobody runs"* — and now it has the number.

▢ **so the wanted shape is PRE-PUSH or CI, not per-turn.** ⚠ and note the cost
GREW with the carve: five crates now, not four. Every future carve adds one.

⭐⭐ **and this is very likely a debt THIS session created**, which is why it is
opened rather than merely noticed. 2026-08-16 deleted or moved an unusual amount:
`ExplosionKind` + three tables, `move_vfx_kind`, `explosion_anim`,
`explosion_sfx`, `WorldManifest`, `LdtkHotReloadState`, `poll_ldtk_file_changes`,
`InventoryRestored`, `DeathRules`'s `Resource` impl, `sync_hosted_sanic_wallet_shield`,
the LDtk `manifest.rs` and `hot_reload.rs`. **Every deletion that leaves a
`` [`Item`] `` reference behind makes one of these.**

⇒ the ratchet's own message states the stake better than a summary can:

> *A deletion that leaves its references behind turns a doc comment into a
> description of a world that no longer exists — **which in this repository is
> where the reasoning lives.***

⚠ that is not decoration. This project keeps its ARGUMENTS in doc comments — why
a thing is shaped as it is, what was tried, what must not be re-derived. A broken
link is a citation into a deleted world, and the next reader cannot tell whether
the claim survived the deletion.

⇒ **the work**: `cargo doc -p <crate> --no-deps`, read the warnings, and for each
one decide whether the reference should be **repointed** (the thing moved) or the
**sentence rewritten** (the thing is gone and the claim needs restating). ⛔ do
not bulk-delete the brackets — that converts a detectable break into an
undetectable stale sentence, which is strictly worse.

⭐ **one worker already showed the right discipline here**: it banked only its own
crate's improvement (`ambition_combat` 17 → 13) rather than laundering the two
rises with `--update`. ⇒ ⛔ **never run `--update` to clear someone else's rise.**

⚠ **and the gating question is the same one D134 raised and left with Jon**
(`awaiting-maintainer-decision.md` item 13). Do not answer it by adding a check;
measure the cost and let him decide.

- ▢ **D136 — COMPOSITION BOUNDARIES ARE ASSUMED, NOT STATED — so whoever
  installs a thing first decides who pays for it. (PROMOTED from `tracks.md`
  2026-08-16, with five instances measured in one night as its evidence)**

⭐⭐ **THREE MORE INSTANCES 2026-08-17, and one of them is the row's thesis
RESOLVED for a single capability — which is what a worked example looks like.**

```text
D152  empowerment EXPIRY was every game's to install, and five games each
      remembered. A sixth that forgot got PERMANENT invulnerability.
      ⇒ resolved: the ENGINE installs the lifecycle in a named set; the ORDER
        stays each game's. "What is engine-owned is the INVARIANT, not the
        order" — which is exactly this row's distinction, stated by the code.
      ⚠ and the honest residue: the five sat in THREE MUTUALLY EXCLUSIVE
        phases, so one shared set has one position and per-game re-placement
        would be a schedule CYCLE. Not every boundary can be stated without
        moving something.

D149  `process_fx_requests` is installed by the HOST, not by the crate that
      writes the channel. So a headless fixture in `ambition_combat` that
      asserted on the visual went BLIND the moment the producer moved onto the
      paired request — the crate could no longer test its own effect.
      ⇒ a capability whose CONSUMER lives above its PRODUCER cannot be
        verified where it is written.

D33   the conversation carve (in flight) is this row in its Cargo form: a
      module with zero outward imports that was nonetheless pinned — first by
      the SCHEDULE (fixed in step 1.5), now by nothing.
```

⇒ **the pattern across all three is that the boundary is discovered by whoever
trips over it**, which is the row's title restated. ⭐ **D152 is the template**:
name the invariant, install it below, leave the ORDER to the composition — and
say out loud which part could not be preserved.

Plan: [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).
Card text: *"Make optional capabilities honest in Cargo dependency closure and
runtime/plugin assembly; a minimal consumer should not silently inherit unrelated
domains."* ⚠ **it was reachable from `tracks.md` and from NO ledger row** — the
same strandedness that made seven designed Engine 1.0 plans invisible on
2026-08-14. Promoted rather than re-derived.

⭐⭐ **it is promoted now because five independent slices on 2026-08-16 all turned
out to be the same failure**, which is a much stronger argument than the card
could make on its own:

```text
D128  the engine cannot ship the art IT draws — every sprite-registration site
      is a GAME system, so `spawn_explosion` reaching for `generic_explosions`
      works only if some game happened to declare it
D132  the durable-save leg is installed by the visible-binary-only presentation
      assembly, so ONE OF TWO persistence authorities does not exist in any
      headless harness — which is why they had never met in a test
D131  a crossover match reads each seat's percent against the pool that seat's
      HOME GAME authored — a DATA value crossing the boundary, not a rule
      (FIXED; the "a demo's rules reach a foreign fighter" reading was wrong)
D134  `runtime → ldtk` was forbidden by two policies nothing ran; the EDGE turned
      out legitimate and downward — but only because a facade deletion converted
      a laundered edge into a declared one, for the THIRD time in that file
D135  the canonical session world carries an authoring-format-specific field, and
      five RON-only games construct `::default()` for a world they never install
```

⇒ **the through-line: none of these is a bug in the ordinary sense.** Each is a
place where *"who is this for?"* was answered by whoever installed it first, and
never written down. ⭐ that is why the composition-shaped slices have been paying
more than the feature-shaped ones this week — naming the schedule order let
`conversation` leave, giving the engine a home for its own art made effects
reachable, and making a load a checkpoint resume removed a whole reconstruction
road.

⚠ **D131 (2026-08-16) sharpened the umbrella and cost it a member.** Its
composition failure was real and was **not a rule reaching a foreign body** — it
was an authored NUMBER (`max_health`) read by another game's rules, so nothing a
system-scoping mechanism could have caught. ⇒ **the umbrella has two shapes, not
one**: (a) a value authored under game A's rules read as universal by game B, and
(b) a global SINGLETON whose owner is whoever installed it last. D131 also
MEASURED an instance of (b) and left it standing on purpose:
`MaryORulesPlugin` inserts `DeathRules::replay_level_after(3.2s)` ungated, after
Sanic's, so **every Smash match in the shipped host runs under Mary-O's death
rules** — inert today (an `Unbounded` fighter writes no `ActorDiedMessage`) and
one composition change from not being. ⛔ two shapes measured is the argument for
a scoping mechanism; one was not, and D131 deliberately did not build one.

⚠ **the standing number to move**: `capability-footprint-may-not-grow` reads
**42 crates linked, 15 a movement-only game never asked for**. ⛔ it has not moved
in any slice yet, and two of them predicted it would not. ⇒ **a slice that claims
this row should say what it did to that number, or say why the number is
dominated by something it did not touch.**

⚠ **D135 was the first executable instance and is DONE (2026-08-16).** ⇒ **and it
answered the standing number above with a NO, which is the more useful answer**:
the footprint did not move, because `ambition_platformer2d_ldtk` is held in a
movement-only game's closure by the MONOLITH — seven production files needing
nine symbols (`WorldManifest`, `LdtkProject`, `ActiveLdtkProject`,
`LdtkHotReloadState`, `poll_ldtk_file_changes`, the `field_*` readers) — and not
by the session world at all. ⇒ **the next instance to take is a monolith carve at
the world-manifest / asset-catalog seam**, not another composition tidy: that is
the one of the nine symbols that is not obviously LDtk-shaped, and until it moves
this counter cannot. This row is still the umbrella; work it by taking that
instance, not by re-measuring the pattern.

⇒ **THE WORLD-MANIFEST INSTANCE LANDED 2026-08-16, AND THE COUNTER STILL READS
42/15 — because the premise above named ONE edge and `cargo tree -i` found
FOUR.** ⛔ *"the monolith holds `ambition_platformer2d_ldtk`"* was true and
incomplete: `ambition_platformer2d` itself declared the backend
**unconditionally**, `ambition_platformer2d_runtime` declares it, and
`ambition_platformer2d_provider` declared it while naming **zero** of its
symbols. Cutting the monolith's edge alone was never going to move this number.

What the slice did:
- `WorldManifest`/`WorldSource`/`world_bevy_asset_path` moved OUT of
  `ambition_platformer2d_ldtk` into `ambition_platformer2d_world::world_manifest`,
  **no re-export left behind**. The type named nothing from the LDtk crate: an
  `AssetId`, four paths/strings, a bool, and a `ron_rooms` field that already
  pointed at the world crate — its sibling `ron_room::RonRoomSource` lived there
  the whole time.
- the provider's **dead** LDtk edge was deleted (measured: no `.rs` file in that
  crate named a symbol from it).
- the facade's LDtk edge is `optional` again and `ldtk_map` is gated on it —
  which the manifest's own ⛔ note had made conditional on exactly this move.
- ⚠ `ambition_platformer2d_world` gained `ambition_asset_manager`
  (`engine.world-ir-dependency-allowlist` amended, not waived). Free by
  measurement: that crate was already in the sentinel's closure, and it is a leaf
  with zero `ambition_*` dependencies, taken without its `bevy` feature.

⭐⭐⭐ **A PLUGIN THAT IS ADDED AND THEN DECLINES TO RUN IS STILL ADDED**
(`d0ed12edb`, 2026-08-16) — this is the sharpening D135 earned and did not get.
D135 made the LDtk spine's six systems decline to RUN in the five RON games, via
`run_if(ldtk_world_installed)`. But the plugin was still ADDED: its six index
resources were still initialized, its systems were still in the schedule graph,
and **`root.ldtk_runtime_index` was still a row in those games' snapshot schema —
the fingerprint two peers must agree on.** ⇒ `run_if` stops EXECUTION; it does not
stop PRESENCE, and presence is what the wire format counts.

⇒ `PlatformerEnginePlugins` no longer adds `LdtkRuntimeSpinePlugin` and
`register_engine_rollback_state` no longer registers the index; both moved behind
`LdtkWorldPlugin`, which **Ambition — the game that actually has an LDtk world —
adds after the engine group.** The row's registration is byte-identical (same
name, kind, projection), so the LDtk composition's schema dump is unchanged and
no schema bump is owed.

⚠ **an honest deferral recorded with it**: the registration lives in the RUNTIME
crate rather than in `ambition_platformer2d_ldtk`, because the floor trait
`RollbackRegistrar` carries only the RESOURCE method and this index is a
COMPONENT on the session root. **Widening the floor is a separate slice**, and
saying so beats a facade.

✔✔ **THE FIRST TWO HOLDERS FELL (2026-08-16, `d0ed12edb` + `a0d452a4c`) — AND
RELOCATION IS NOW EXHAUSTED, WHICH IS THE FINDING.**

⭐ **`WorldManifest` and the hot-reload watcher are both off the monolith's list,
and what remains is ALL GENUINELY LDtk** — five production files needing
`LdtkProject`, `LdtkLevel`, `ActiveLdtkProject`, `LdtkVocabulary` and the
`field_*` accessors (`encounter/loading.rs`, `encounter/systems.rs`,
`menu/map/systems.rs`, `world/gated_lock_walls.rs`, `world/mod.rs`). ⇒ **no
further RELOCATION can cure this.** The remaining cure is **INVERSION** — route
those readers onto the room IR instead of the project — and the encounter pair is
the LARGE one, already planned in its own comment as *"W4 will route encounter
loading through RoomEmission instead of the project"*. **It is now the only thing
between the workspace and 42/15.**

⚠ **the row said FOUR holders; `cargo tree -i` names TWO** — the monolith and the
runtime. The other two were transitive re-listings of the same crates. Corrected
by the tool, again.

⚠ **and the runtime's edge is now legitimate by design**: it owns `LdtkWorldPlugin`
and the LDtk rollback domain, which a game ADDS if it wants them. That is a
declared offer, not an imposition.

⇒ **what still holds the edge, and the cost of each**, in the order they must
fall (the last two cannot be cfg-gated cheaply — the code has to move):

```text
runtime    LdtkRuntimeSpinePlugin is in PlatformerEnginePlugins unconditionally,
           and LdtkRuntimeIndex is rollback-registered there. SMALL, and it is
           the exact successor to D135 — a format installs its own spine.
monolith   LdtkHotReloadState + poll_ldtk_file_changes, in features/mod.rs and
           persistence/settings/model. ~35 refs. NOT LDtk: a debounced mtime
           watcher over an Option<PathBuf> whose ctor takes an asset catalog.
monolith   menu/map/systems.rs builds map nodes by walking LDtk levels. It wants
           room metadata instead; needs a world rect on the room. MEDIUM.
monolith   world/gated_lock_walls.rs walks the project for LockWall entities.
           MEDIUM — the same inversion, onto the room IR.
monolith   encounter/loading.rs + encounter/systems.rs read LDtk levels/fields
           to build encounter specs. LARGE, and already planned: the comment in
           systems.rs says "W4 will route encounter loading through RoomEmission
           instead of the project".
```

⭐ **the sortable finding for the umbrella**: of the nine symbols, only THREE are
genuine format vocabulary held by production code (`LdtkProject`,
`ActiveLdtkProject`, `LdtkLevel`, reached through `field_string`/`field_f32`);
`LdtkVocabulary` is named by a TEST only; and `WorldManifest`,
`LdtkHotReloadState` and `poll_ldtk_file_changes` are engine concepts wearing an
LDtk name. One of the three is now gone. **⛔ the lesson for the next instance is
the measurement, not the move: run `cargo tree -i` for the crate you mean to
evict BEFORE choosing which code to carve.**

- ✔ **D135 — THE CANONICAL SESSION WORLD CARRIES AN AUTHORING-FORMAT-SPECIFIC
  FIELD, AND FIVE GAMES FILL IT WITH `::default()`.
  (opened 2026-08-16, split out of D134; DONE 2026-08-16 — read the ⇒ closure at
  the end of this row, which chose shape (2) and struck two of the row's own
  numbers)**

`PlatformerSessionWorld` has `runtime_rooms: LdtkRuntimeIndex`, and
`PreparedPlatformerSource` carries it through four public constructors. Sanic,
Mary-O, Pocket, TwinTrack and Smash are RON-authored and every one of them
constructs `LdtkRuntimeIndex::default()` for a world it will never install — the
type's own `Default` doc says so out loud (*"the 'no LDtk world installed'
index"*). ⇒ **a format adapter's type is a member of the engine's canonical
session world**, which is the thing the deny row was pointing at before D134
established that the manifest edge itself is legitimate.

⭐ **it is smaller than it looks, and the measurement is why**: the struct's
fields are all `std` (`String`, two `BTreeMap`s, two `u64`) plus a plain POD, and
**exactly one method touches `bevy_ecs_ldtk`** — `level_set_for` returning
`LevelSet`, called from exactly one place, inside the LDtk crate itself
(`bevy_runtime/asset.rs`). The rollback domain needs only `Component + Clone` and
`active_area() -> &str`.

⇒ **the shape**: move the struct down to `ambition_platformer2d_world` (a shared
dependency of ldtk, runtime, monolith, provider and content, so it is cycle-free —
⛔ moving it UP into the runtime is not, since `monolith → ldtk` and
`provider → ldtk` both exist), rename it for what it is (an active-area index, not
an LDtk one), and leave `level_set_for` / `from_project` behind in the LDtk crate
as free functions over the moved type. **The cost is the rename**: 58 `.rs`
references across 15 crates, of which ~25 are `::default()` in demo/provider
construction.

⚠ **do not start this by moving the type.** Start by asking whether a RON-only
game should carry the field at all — if the honest answer is "no", the fix is a
different shape (the index becomes optional session state a format installs)
and the rename is wasted work.

⇒ ✔✔ **THE ANSWER WAS "NO", SHAPE (2) LANDED, AND THE MOVE+RENAME WAS NEVER
STARTED — 2026-08-16.** The row's own warning was worth every line of it: the
58-site rename would have been wasted work.

⭐ **the measurement that decided it — who reads the field, on what road.** Ten
readers, and only two are engine-side. **(a) FIVE are LDtk's own systems**
(`sync_ldtk_level_set` plus the four `rebuild_ldtk_runtime_*_index`), all in
`ambition_platformer2d_ldtk`, all reading it through a `SessionWorldRef`.
**(b) the monolith's `SimulationSetup::ldtk_index` WAS DEAD** — borrowed,
silenced with `let _ = ldtk_index;`, read by nothing, and its comment promised a
"follow-up patch" that never came. It is the reason the field looked like
something simulation setup needed. **(c) the provider's content digest** writes a
`world.runtime-index` section that, for a RON game, is one `area\t<id>\t\t-` row
per room — i.e. nothing. **(d) the rollback checksum** hashes `active_area()`,
which mirrors `RoomSet::active_spec().id` that `root.room_set` already checksums.
⇒ nothing engine-side needs an active-area index regardless of format. **And
exactly ONE site in the workspace ever built a non-default index**
(`from_project`, in `ambition_app`). Shape (1) was indicated by nothing.

⇒ **what shipped**: `PreparedPlatformerSource` holds `Option<LdtkRuntimeIndex>`,
private, `None` by default; `new` / `for_match` / `with_world` LOST the
parameter; the LDtk road states its own installation with
`.with_installed_ldtk_index(..)`; and `PlatformerSessionWorld` — the canonical
bundle — no longer carries the field at all. The provider inserts the component
onto the session root only when a format installed one, so a RON-authored root
carries nothing for it.

⭐ **and the deletion went further than the field, because the field was
propping up three other things.** ⇥ `ambition_platformer2d_runtime::demo_fixture`
re-exported `LdtkRuntimeIndex` **so RON demo shells could hand an empty index to
a constructor that demanded one** — a fixture module laundering a format
dependency into games that have none. Gone. ⇥ **three setup systems — Sanic's,
Mary-O's and `ambition_app`'s — took a `SessionWorldRef<LdtkRuntimeIndex>`
parameter purely to forward it to the dead monolith field.** Two of those are
RON games *querying the session root for LDtk state in order to boot*; had the
component simply been made absent without removing them, `Single` validation
would have failed and neither demo would have started. Gone. ⇥ and
`LdtkRuntimeSpinePlugin`'s six-system chain now carries
`.run_if(ldtk_world_installed)`: every game composing the runtime plugin group
used to run all six each sim tick, and in the five RON games they rebuilt against
an empty index and found nothing, forever. **The optional field is what finally
made that statable.**

⛔ **two of this row's own numbers were wrong, struck here rather than quietly
fixed.** ⊘ *"~25 `::default()` in demo/provider construction"* — it is **14**,
and 4 of those were `#[cfg(test)]` fixtures inside the provider. The overcount
came from a grep that swept `.claude/worktrees/` clones. ⊘ *"58 `.rs` references
across 15 crates"* — the real workspace figure is 56, same cause. ⇒ the honest
deletion is **14 `::default()` constructions, every one of them gone**, plus the
dead monolith parameter, three system parameters, and the `demo_fixture`
re-export. Five games and the provider's test fixtures no longer name
`LdtkRuntimeIndex` at all; `ambition_platformer2d_provider/src/lifecycle.rs`
names it **zero** times.

⛔⛔ **`capability-footprint-may-not-grow` DID NOT MOVE, and the row asked the
right question in asking.** Still **42 crates linked, 15 a movement-only game
never asked for**, with `ambition_platformer2d_ldtk` still among the fifteen. ⇒
**the footprint is dominated by the MONOLITH, not by the session world.** Seven
production files under `ambition_platformer2d_actor_monolith/src` still need nine
distinct symbols from that crate — `WorldManifest` (the asset catalog),
`LdtkProject` + `LdtkLevel` + `field_string`/`field_f32` (encounter loading and
gated lock walls), `ActiveLdtkProject` (encounter systems, the map menu),
`LdtkHotReloadState` + `poll_ldtk_file_changes` (hot reload, settings) and
`LdtkVocabulary`. `LdtkRuntimeIndex` was never what held that edge, so removing
it could not have moved the counter. ⚠ **that is worth more than a moved number**:
it says the next slice against this ratchet is a monolith carve — specifically
the world-manifest/asset-catalog seam, which is the only one of the nine that is
not obviously LDtk-shaped — and not another session-world tidy.

⇒ **the guard**: `game/ambition_app/tests/a_ron_game_installs_no_ldtk_world.rs`,
two tests that are one claim. ⛔ *"Sanic has no LDtk index"* is trivially
satisfied by never inserting the component anywhere — which is what a bad
implementation of this change looks like, and it would delete level streaming
from the shipped game while turning the file green. So the absence is asserted
only beside a POSITIVE observation that the LDtk-authored game installs a real,
**non-empty** index (`active_area()` is the field `from_project` fills and
`Default` leaves blank, so it separates an installation from the placeholder).
Both host-driven. ⚠ **the negative fixture was CAUGHT not reaching its state**:
written with `.start_at_launcher()` it never activated a session in 240 frames
and said so rather than passing vacuously. Falsified by restoring the old
behaviour (`insert(installed_ldtk_index.unwrap_or_default())`) — red, with
`session-start experience=ambition_versus` / `room-loaded versus_arena` in the
trace proving it reached a live session first — then restored and re-run green.

⚠ **rollback schema stayed at 34 and needed no bump, verified rather than
assumed**: the fingerprint is computed from the REGISTRATION list
(`registry.rs::schema_dump`), no registration changed, and no coverage sweep
fails on a registered component being ABSENT — every sweep runs
entity→registry, so a registered type carried by nobody contributes no entity.
(`inert_registrations` is the near miss and is also presence-driven.) The content
digest is likewise unmoved: the section a `None` index writes is byte-identical
to what an empty index wrote, which is why no fingerprint shifted.

⚠ **one thing the D134 note got right and this row should not lose**: the fix did
not touch a single manifest edge. The runtime still declares
`ambition_platformer2d_ldtk`, correctly. A dependency-denylist row could not have
stated this wart, and removing the wart did not move a dependency count — two
instruments, two facts.

**D134 as it was OPENED, kept for the record. ⛔ two of its claims did not
survive contact and are struck below** — read the closure above first:

- ⊘ *"a `std` hash container … is the exact defect ADR 0023 exists to forbid"* —
  the site sorted before folding, so no observable ever saw the hash order. The
  hazard was real; the defect was not.
- ⊘ *"an upward dependency the policy denies twice over"* — `cargo tree` says the
  ldtk crate sits BELOW the runtime and below the monolith. The policy was wrong,
  not the manifest.
- ⊘ *"NOTHING RUNS IT"* — `./run_tests.sh` does, via `cargo test --workspace`.
  What no gate runs is the suite **on the turn that breaks it**.

⛔⛔ **`cargo test -p ambition_workspace_policy` fails with 12 violations in the
`engine` scope, and NOTHING RUNS IT.** The standing gate is
`cargo check -p ambition_app --all-targets` + the app suite + Smash; this suite is
in none of them. ⇒ a set of invariants each carrying an `owners` list, a
`source_doc` and a written rationale has been failing unobserved. ⚠ pre-existing —
none of them belongs to the work landed tonight.

**The 12, by policy** (`tests/ambition_workspace_policy/policies/engine.toml`):

```text
engine.determinism                     (ADR 0023)
  world/src/rooms/gate_portal.rs:199   iterates `phases`, a std hash container —
                                       RandomState order differs BETWEEN RUNS
engine.movement-model-is-never-optional (ADR 0024 §1)
  features/ecs/actors/update.rs:237    names Option<&MotionModel>
engine.player-fallback-update-documented
  features/ecs/actors/update.rs        must contain AMBITION_REVIEW(determinism)
engine.pose-writes-are-authority-only
  features/ecs/actor_clusters.rs:686   seed.kin.pos = start;
engine.velocity-writes-are-authority-only
  characters/src/brain/fighter/recovery.rs:286   body.kinematics.vel = at.vel;
engine.runtime-manifest-allow / -deny / runtime-source-no-upper   (7 sites)
  runtime -> ambition_platformer2d_ldtk, a dependency the manifest both fails to
  allow and explicitly denies, plus 5 source spellings of it
```

⭐⭐ **the determinism one is the reason this row is not merely housekeeping**: a
`std` hash container iterated in `gate_portal` is the exact defect ADR 0023
exists to forbid, and it differs **between runs of the same build** — which is
what every rollback checksum and every replay claim in this repo assumes cannot
happen. ⚠ the `runtime → ldtk` cluster is 7 of the 12 and is one architectural
fact, not seven: an upward dependency the policy denies twice over.

⇒ **two things are owed and they belong to DIFFERENT PEOPLE**: (a) fixing the
violations is work, and this row owns it; (b) **whether the suite joins the goal
guard is JON'S, and it is already logged** as item 13 of
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) — where a
previous session deliberately declined to act, in these words: *"the guard's
check list is yours, and adding a red check would stop every autonomous run until
the twelve are cleared."*

⛔⛔ **this row was opened without reading that file first, and the brief it
produced told an agent to decide (b) and implement it.** That is answering a
reserved question by dispatch, and the hazard is concrete rather than procedural:
`.goal/active.json`'s check list is what keeps an autonomous run alive, so a red
check added there **wedges every turn** until all twelve clear. Corrected
mid-flight. ⇒ **promote the WORK from that file, never the DECISION**, and say in
the row which half is which.

⭐ that file also carries a **better diagnosis than this row first had**: the
`engine.determinism` hit is a **false positive on correct code** — the site
`collect`s and then **sorts** — so the honest fix is structural (make `phases` a
`BTreeMap`, making ordered iteration a property of the TYPE rather than a
discipline the next editor can drop), and *"a waiver would be the wrong answer
here."*

### ⭐⭐ SHAPE (b) IS FIXED, AND THE SCOPING MECHANISM ALREADY EXISTED — IN THE WRONG CRATE FOR IT (2026-08-16)

**The question the row set was *"why does `DeathRules` not use
`ExperienceScopeBuilder`?"* with three candidate answers. The measured answer is
(2), and specifically:** the shell's scope builder **has adopters** — smash
(`demo_smash/src/lib.rs:1660`) and versus (`app/versus.rs:1034`) both declare
scopes, and a policy test already reads them all at once
(`experience_scope_ownership.rs`) — so *"nobody adopted it"* is FALSE. It does
not fit because it releases state a SESSION published on route DEPARTURE, and it
has no entering half at all (*"entering is not an event anything has to catch"*).
`DeathRules` was inserted in `Plugin::build` and lives for the process; releasing
it on the first departure would delete it forever.

⭐⭐ **AND THE RIGHT MECHANISM ALSO ALREADY EXISTED, ONE NOUN SHORT.**
`ambition_platformer2d_runtime::mode_scope` **is** the demo-hosting seam: it
scopes a hosted game's SYSTEMS (`in_mode` / `in_base_mode`) and its ENTITIES
(`ModeScopedEntity`) to the rooms tagged with its mode, and **every one of
Sanic's and Mary-O's systems is already gated through it**. It had no word for a
RULE. ⇒ so three games each inserted a process-global instead, and the last
`Plugin::build` won — which is not a missing mechanism, it is a mechanism with a
missing third noun.

⇒ **`DeathRules` stopped being a `Resource`.** A game declares into
`DeclaredDeathRules` under the rooms it governs
(`DeathRulesScope::{Mode(&str), UntaggedRooms, EveryRoom}` — the same three
answers `<Demo>RulesPlugin` already gives when it decides how to gate its
systems, selected by the SAME `hosted` constructor flag). `governing(mode)` is
the one place *"whose rules apply here?"* is answered, and it is reached through
one `SystemParam` (`GoverningDeathRules`) so a third beat cannot re-derive it.
**A room no game claimed reads `DeathRules::default()` — `LevelReset::Never`,
which is exactly what an arena wants.** A second declaration of one scope panics
at build rather than picking a winner.

⚠ **the mode tag was already universal and nobody had noticed**: smash, versus,
twintrack, pocket, sanic and mary_o all tag their rooms; only Ambition's own are
untagged. The fix needed no new identity.

⛔ **the wallet-shield needed a DIFFERENT cure, and a smaller one.**
`sync_hosted_sanic_wallet_shield` is not a global — it is a system whose
POPULATION was every `PrimaryPlayer` in the process. It was inert only because
`BodyWalletShield` has exactly one writer in the workspace (measured: this file).
That claim is now written down and the loop states its population: a body is
Sanic's business when it wears a Sanic persona or already carries a shield, and a
George Booul on a Smash stage is neither. ⇒ **and the `hosted` fork on that
system is DELETED** — two systems and a bool became one, because the standalone
binary loads the same `mode: Some("sanic")` speedway the host does, so the
constructor flag was answering a question the ROOM already answers.

**Rollback: schema did NOT move (still 34).** `DeathRules` was never
`require_rollback`-registered — it was a WAIVED row in `rollback_coverage.rs`,
and the waiver moved to `DeclaredDeathRules` with its argument intact
(`declare` is only reachable through `App`; a tick holds a `World`). ⚠ recorded
with it: that argument would NOT survive a resolved-rules resource written each
tick, which is why the resolution is a `SystemParam` that stores nothing.

⇒ **WHAT A THIRD INSTANCE WOULD HAVE TO LOOK LIKE to justify a general
mechanism** — because this slice deliberately did not build one. Not another
resource: a resource is now cured by *"declare it into a collection keyed by the
rooms you govern"*, which is a five-line pattern, not a framework. The instance
that would justify one is **a game that must scope something the mode tag cannot
express** — state belonging to a game while it is NOT in its own rooms (a
crossover fighter carrying its home game's rules onto a foreign stage), or a
scope whose boundary is a SEAT rather than a room. That is the day
`ExperienceScope` and `mode_scope` have to become one vocabulary instead of two;
today they are two because they scope two different lifetimes, and saying so is
cheaper than merging them.

⚠ **stale in the brief that opened this, for the record**: it said `DeathRules`
is *"rollback-registered (`rollback_coverage.rs:987`)"*. Line 987 is the WAIVER
list, not a registration — the distinction is what made the schema question a
non-event.

- ☑ **D131 — FIXED: NOTHING ACCRUED ON A CLOCK. FOUR FIGHTERS WERE BEING
  DIVIDED BY 1, 1, 60 AND 100. (opened + closed 2026-08-16)**

### ⭐⭐ THE ANSWER: THE DENOMINATOR, NOT THE NUMERATOR

**`damage_percent()` is `accumulated / max`, and `max` was each character's
HOME GAME's authored pool.** Reproduced headlessly through the shipped shell
(4 CPU seats, `smash_gameplay`), logging every `HitEvent` with its source:

```text
mary_o             42 damage / max   1  = 4200%   <- Jon's exact number, ~same tick
sanic               8 damage / max   1  =  800%
player_robot_v3    11 damage / max  60  =   18%
smash_george_booul  9 damage / max 100  =    9%
```

⇒ **every hit was `HitSource::Melee` from the other fighters.** The meter was
honest and the division was correct. Mary-O and Sanic author `max_health: 1`
because they are one-hit-kill platformer protagonists — true in their own games,
and it makes one point of ordinary damage read as a full meter on a stocks stage.

⛔⛔ **and the swap-to-P2 control was right and pointed at the wrong thing.** It
proved the cause travels with the CHARACTER — and what travels with a character
is its authored VITALS, not a system. **No demo system was involved.** The
crossover hypothesis (a `SanicExperiencePlugin` / `MaryOExperiencePlugin` system
damaging a foreign body) is **FALSIFIED**: measured in the same run, `players=0`,
`outofplay=0`, zero `ActorDiedMessage`, zero `RoomReplayRequested`. What crossed
the boundary was a NUMBER, not a rule.

⚠ **the standalone-app experiment would have "confirmed" the false hypothesis.**
`ambition_demo_smash_app` composes no Mary-O or Sanic provider, so
`SmashRoster::assemble` drops both and its whole cast was stamped with the
reference — the bug is structurally unreachable there, for a reason unrelated to
which plugins are installed.

### THE FIX — the MATCH declares what 100% means

`MatchParticipantRoster::fighter_health_pool: Option<i32>` →
`MatchRules::health_pool` → `MatchRules::pool_over(authored)`, applied at both
seat sites in `prepared_match.rs` (spawned seed AND adopted `body.max_health`).
`apply_smash_match_rules` declares `SMASH_PERCENT_REFERENCE`.

⭐ **the tell it was half a decision:** a stocks match already overruled the
character on *whether the pool kills* (`MatchRules::death_policy`) and left *how
big the pool is* with the character. `pool_over` sits one function above
`death_policy` for that reason.

⛔ **DELETED: the per-character workaround.** `definition.vitals.max_health =
Some(SMASH_PERCENT_REFERENCE)` on the three ids this demo registers (2026-07-31,
the 14000% fix). It was right about the symptom and wrong about the owner — it
could only ever reach three fighters and the roster is fourteen. Its guard test
`each_duelist_authors_the_pool_its_percent_is_read_against` is replaced by
`the_match_declares_the_pool_every_fighters_percent_is_read_against`.

### ⚠ THE SECOND DEFECT WAS THE SAME INSTRUMENT, NOT A SECOND BUG

**"Both fighters lose a stock at the same instant while one is at 0%"** — the KOs
are 3 and 24 ticks apart in the trace, which at 60-frame sampling is one instant;
and `spend_fighter_stocks` calls `health.reset()` on a fighter coming back, so a
seat that just lost a stock reads **0% because it respawned**. Correct behaviour
seen at capture resolution.

### ⛔ WHAT THE HUNT FOUND ON THE WAY — a REAL crossover leak, measured INERT

`MaryORulesPlugin::build` inserts `DeathRules::replay_level_after(3.2s)` as a
GLOBAL resource with no gate (`demo_mary_o/src/lib.rs:1941`), Sanic inserts its
own (`demo_sanic/src/lib.rs:1163`), and `shell_host.rs` installs Mary-O AFTER
Sanic — so **every Smash match in the shipped host runs under Mary-O's death
rules**, whose own doc says a versus arena wants `LevelReset::Never`. Measured
inert today: an `Unbounded` fighter never writes `ActorDiedMessage` and a seat
carries no `PlayerEntity`, so `open_death_interlude` never fires. ⇒ **not fixed
here, and it is D136's second instance in a different shape** — the first was a
DATA value authored under one game's rules, this is a global SINGLETON whose
owner is whoever installed it last. Two shapes is what would justify a scoping
mechanism; one would not have.
⚠ same file, same shape, also ungated: `sync_hosted_sanic_wallet_shield`
(`demo_sanic/src/lib.rs:1225`) runs every tick of a Smash match and its non-Sanic
branch REMOVES `BodyWalletShield` from any `PrimaryPlayer`.

### the original report, kept

⛔⛔ **the Smash showcase is not currently a fight.** Sanic passes 200% before the
`GO!` banner clears and 600% by six seconds; Mary-O reaches **4200% in six
seconds** against a George Booul floating several body-widths off-stage with no
contact at any point. Meanwhile `player_robot_v3` and `george_booul` read **0% in
every single sample**.

⭐⭐ **THE CONTROL IS WHAT MAKES THIS A DIAGNOSIS RATHER THAN A SIGHTING.** Swap
Sanic to P2 and **the damage moves with him** — the identical 600% at frame 360,
against a different opponent. ⇒ in one experiment this eliminates the seat, the
opponent, the stage position and the matchup. **Percent advances per frame, per
character, with the attacker uninvolved.**

⚠ **and it splits the cast two-and-two**, which is a clue and not yet an answer:

```text
runs away    sanic, mary_o                 (each the star of its own demo)
never hit    player_robot_v3, george_booul (native to Ambition / Smash)
```

⇒ crossover-vs-native is the obvious hypothesis and **two a side is too small to
believe it** — widen the sample across the other eleven fighters before chasing
it. ⛔ do not start from the characters; start from **whatever advances damage per
frame** and ask what admits these two and not those two.

⛔ **TWO LEADS ALREADY DEAD (2026-08-16) — do not re-run them:**

1. ⛔⛔ **RETRACTED 2026-08-16 — I ELIMINATED THIS AGAINST THE WRONG PROCESS, AND
   IT IS NOW THE LEADING HYPOTHESIS.** I checked that `ambition_demo_smash` and
   `ambition_demo_smash_app` name neither `demo_sanic` nor `demo_mary_o` — true,
   and irrelevant: **the measurement was taken through the SHELL**
   (`capture_scene --route smash_gameplay`, i.e. `ambition_app`), and
   `app/shell_host.rs` installs **`SanicExperiencePlugin` and
   `MaryOExperiencePlugin`** outright. ⇒ both demos' systems WERE running in the
   process where the damage was observed.

   ⭐⭐ **and it explains the split exactly**: `SMASH_ROSTER`'s own comment calls
   `mary_o` and `sanic` *"the other demos' protagonists — present only when a host
   composes them"*, while `player_robot_v3` and `george_booul` belong to Ambition
   and this demo. **The two that accrue damage are precisely the two whose
   experience plugins are composed into that process.** ⇒ look for a system in
   `SanicExperiencePlugin` / `MaryOExperiencePlugin` that damages its own
   protagonist on a clock and does not check that it is in ITS OWN experience —
   a hazard, a drown/lava timer, a form-loss rule.
   ⚠ **and this predicts the standalone `ambition_demo_smash_app` would NOT show
   it**, which is a cheap confirming experiment before touching any code.

   ⚠ **two narrowings from a first pass, so the next one starts further along:**
   - ⊘ **it is not an emitted HIT from either demo crate.** The only
     `HitEvent` writer in both is `demo_mary_o/src/snake.rs` (an enemy's
     contact), and neither crate emits anything per-frame at a protagonist.
     ⇒ suspect a **direct write to the body's damage**, or a shared engine
     system the demo plugin *configures* rather than one the demo owns.
   - ⭐ **whatever it is keys on CHARACTER IDENTITY, not on the seat or on
     "the player"** — that is what the swap-to-P2 control proves, and it is a
     strong filter: look for a system whose query or lookup names
     `SANIC_CHARACTER_ID` / the Mary-O equivalent, or reads `WornCharacter`,
     rather than one that acts on `PrimaryPlayer` or `ControlledSubject`.
2. ⊘ **"the two that accrue are the two whose sheets do not author a body."**
   Falsified 4/4: `george_booul` is ALSO `authored_body: false` and takes **zero**
   damage in every sample, while `player_robot_v3` is `true`. Body-geometry
   provenance does not split the cast the way the damage does.

⭐ **what the control does establish, and it narrows the search a lot**: the
character that accrues is the VICTIM, and the opponent is uninvolved — so the
source is **the character's own body or the stage**, not an attack resolving from
the other side. Look for something that emits a hit, or writes damage, per frame
at a body, and admits `sanic`/`mary_o` while refusing `player_robot_v3`/
`george_booul`. ⚠ D128 records the shape of a near neighbour already fixed once:
a swing broadcast a body-scanning volume that **came back around to its owner**.

⚠ pair it with the second defect in the same match: **both fighters lose a stock
at the same instant while one of them is at 0%.**

⭐ reproducible now, which is the only reason any of this is visible — see D130
for the nine-tap command; sampled at 240/300/360/420/600/900/1400/2400 frames.

- ☑ **D130 — (b) FIXED: the instrument can now photograph a real match. (a) was
  a MISDIAGNOSIS: there is no tofu. (opened + closed 2026-08-16 by LOOKING)**

### (a) ⛔⛔ THERE IS NO TOFU. IT IS THE STAGE FLOOR.

**The "two full lines of ~35 hollow boxes" are the Smash stage's TILES**, and
the "blank grey rounded HUD chips above them" are **blurred rounded rectangles
in the far parallax backdrop** — a distant lit cityscape. Neither is text.
Photographed at 3x: each box has a bevelled top-left highlight and a dark border,
i.e. a platform tile, not a `.notdef` glyph.

⭐ **why it read as tofu**: `--route smash_gameplay` with no roster puts the
camera at the default position with no subject to follow, so the stage's floor
sits alone at the bottom-right of an empty frame with no fighter, no HUD text
and no scale cue next to it. Two neat rows of ~35 identical squares in a corner
of a blank screen is a very good impression of two lines of tofu.

⇒ **`crates/ambition_render/src/hud/declared.rs`'s font fallback is INNOCENT.**
In a real match every string renders correctly at 1280x720: `Sanic 200% · 3/3`,
`Player Robot v3 0% · 3/3`, the `GO!` banner, both nameplates — including the
`·` (U+00B7). ⛔ do not re-open this on the strength of the original report.

### (b) ⭐ FIXED — `capture_scene` grew the step that carries a POSITION

`--press touch:XxY` sends the pair of real `TouchInput` messages winit emits and
lets Bevy's own `touch_screen_input_system` fold them, so the tool drives the
PHONE road the product ships. Generic, not a `--smash-cpu` flag: any route that
answers a finger gets it.

⭐ **the cause, confirmed**: key taps are EDGES WITH NO POSITION. The tests seat
a fighter with `SelectCursor::move_to(rect.center())` and THEN `tap(Enter)`; the
tool's bare `Enter` fired wherever the cursor sat. The doc block claiming the
two were "exactly" the same is corrected in place.

⚠ **`the_arrows_alone_can_work_the_whole_screen` is NOT in conflict.** Arrows do
work — the cursor starts on portrait 0 and snaps — `Down,Enter,Enter` was simply
not a seating sequence (one Down lands on another portrait). Arrows are a maze
here, not a wall.

The working command is in `capture_scene`'s header and guarded by
`smash_in_the_host::the_capture_tools_documented_taps_seat_two_cpus_on_two_fighters`,
which drives the same literals through the real host.

### ⭐⭐ AND THE MATCH, WATCHED — the percent runs away on its own

Three CPU-vs-CPU matches, sampled by frame. All 1280x720, `--include-ui`.

```text
P1 vs P2                       frame  P1              P2
Sanic      vs Player Robot v3   240   200%  3/3       0%  3/3   ("GO!" still up)
                                300   500%  3/3       0%  3/3
                                360   600%  3/3       0%  3/3
                                420     0%  2/3       0%  2/3
                                600     0%  1/3       0%  1/3
                                900   400%  1/3       0%  1/3
                               1400   (eliminated)    0%  1/3
                               2400   back on select, all four slots reset
Mary-O     vs George Booul      360  4200%  3/3       0%  3/3
Player Robot v3 vs Sanic        360     0%  3/3     600%  3/3   ⭐ THE CONTROL
```

1. **⛔⛔ THE PERCENT IS NOT COMING FROM THE OPPONENT.** ⭐ **the seat swap is
   the control**: move Sanic from P1 to P2 and the damage moves WITH HIM, and
   he reads **600% at frame 360 in both runs — the identical number**, against
   two different opponents doing two different things. In the Mary-O run the two
   fighters are not even near each other (Mary-O on the platform's right edge,
   George Booul floating off-stage several body-widths away) and Mary-O is at
   **4200%** in six seconds. ⇒ this accrues on a CLOCK, per character, with no
   opponent involvement.
2. **⛔ AND IT SPLITS THE CAST IN HALF.** `player_robot_v3` and `george_booul`
   read **0% in every sample of every match**; `sanic` and `mary_o` run away.
   ⚠ that is the demo's own fighters versus the CROSSOVER cast, which is the
   first thing to check — but two characters a side is a small sample, so widen
   it before believing the split.
3. **⛔ BOTH LOSE A STOCK AT THE SAME MOMENT, and one of them is at 0%.** In the
   Sanic run 3/3 → 2/3 → 1/3 happens to BOTH between the same samples while the
   robot has taken no damage at all. Then they diverge (Sanic is eliminated by
   1400, the robot keeps its last stock), so it is not one shared counter — but
   two simultaneous KOs where only one fighter was ever damaged wants explaining.

⭐ what is RIGHT: the stage draws, the fighters draw at sane relative scale,
both kits animate (the robot is mid-swing with a visible purple weapon at 900),
the HUD is legible and correct, the match ENDS on its own and returns to a fully
reset select screen. Nothing is missing; the fight itself is wrong.

⇒ **this is D128's "does a watcher SEE the two kits behave differently" cashing
in, and the answer is worse than 'no': half the cast is on fire from the first
frame.** Next row is (1)+(2) together — start from whatever ticks percent
without a hit.

- ▢ **D129 — The sprite pipeline CUTS ART AT THE LOGICAL FRAME AND NOTHING NOTICES.
  (opened 2026-08-16 from a maintainer observation, measured the same day)**

Jon: *"Super sanics spikes are clipped by the sprite renderer. This might need a
structural fix. We should not be able to clip sprite artwork so easily."*
⇒ **true, and it is not one character.**

✔✔ **GUARD LANDED 2026-08-16** (renderer `6228c58`) — the renderer now WARNS,
at draw time, when a frame's drawing runs off the logical frame, naming the
animation, frame and edges.

⛔⛔ **AND THE "52 SHEETS" NUMBER CANNOT BE CASUALLY REFRESHED — measured
2026-08-17, and it changes how this row should be worked.** `clipped_frame_edges`
is called from `sheet_build.py:943`, **during a BUILD, on the drawing canvas
BEFORE padding** — and its own comment says why: *"this is the only place a
clipped edge is still visible."* The shipped PNG no longer carries the evidence,
because the frame it would be measured from is the one the packer already
trimmed.

⇒ **the count is a BUILD-TIME observation, not a property of the repository.**
Re-measuring it means a full regen of all 196 sheets, which rewrites gitignored
art — so `52 of 196` is a SNAPSHOT dated 2026-08-16, and anyone quoting it later
should say so rather than treat it as current.
⭐ **the practical consequence: this row is closed by REDRAWING, and the
measurement comes free with the redraw.** Nobody needs to re-run a survey first —
the guard fires on the next build of any sheet somebody touches, which is exactly
when the number matters.
⚠ this also explains why the row could not simply be made fatal: a fatal check at
build time would block regenerating ANY sheet until the whole roster was redrawn,
which is the deadlock its author declined. It warns rather than raises because 52 sheets already
trip it and a fatal check would stop everyone regenerating anything until the
roster is redrawn; whoever fixes the art can make it fatal. Seven tests.

⛔⛔ **AND THE FIRST TWO CRITERIA WERE BOTH WRONG — including the one this row
originally published.**

1. *"the art touches a logical-frame boundary"* → flags **74 of 133**. Useless:
   with `auto_crop` the frame is FITTED to the art, so touching is the normal
   case.
2. *"…and has ≥6 opaque pixels in a straight run covering >25% of that edge"* →
   **this row's original criterion, and it hides a denominator.** *Wide relative
   to what?* Against the trimmed rect's width it flags `super_sanic`; against the
   logical frame's width it does not. Nothing chooses between them. ⇒ the
   **"23 sheets" published here was not trustworthy**, and it was caught only by
   building the guard and watching it disagree with the scan that produced it.

⭐ **the criterion that survives is denominator-free, and it is the one thing
actually measured: a truncated shape does not TAPER.** Compare the edge line to
the widest the shape reaches within a few lines inside it — a tip narrows on its
way out, a cut arrives already near full width.

**Re-measured with it: 52 of 196 sheets, with frame counts.** Worst first —
`ninja_shadow_oni_leader` 73 frames (all four edges), `ninja_shadow_duelist` 70
(all four), `player_combat_review` 108, `player_traversal_review` 100,
`trex_enemy` 57 (bottom+left), **`super_sanic` 54 (top — Jon's report)**,
`raid_enforcer` 52, `fascist_enforcer` 53, `pulse_voyager_captain` 48,
`perfect_cellular_automaton` 45, `goblin_shaman_staff` 39, `tech_bro_disruptor`
and `goblin_cantina_chieftain` 35, `robot` 34, `robot_guardian` 33,
`m_leblanc` 32, `player_extended` 30, and 35 more with fewer frames each
(`pirate_admiral` only 2, `oiler_vfx` 1).

⭐ **the CONTROL is what makes it causal**: base `sanic` is clean and only
`super_sanic` is cut, and the super skin is the same body with `spikes_up=True`.
Spikes down, fine; spikes up, cut.

⭐⭐ **and 23 sheets are far fewer than 23 CAUSES — they collapse by source
YAML.** Eight of them (`robot`, `player_extended`, both player `*_review` sheets,
`robot_caster`/`diver`/`miner`/`runner`) are all auto-emitted from
`robot_spritesheet.yaml`; the two `sandbag_*_review` come from
`sandbag_spritesheet.yaml`; `ninja_shadow_oni_leader` and `ninja_shadow_duelist`
share `ninja_spritesheet.yaml`. ⇒ **~15 authoring sources, and the single largest
is the robot family**, whose cut edge is `top` — where
`player_robot_v3.py` records the antenna spike sitting. `robot_spritesheet.png`
is EMBEDDED in `ambition_asset_manager`, so this ships. ⚠ the current player
draws `player_robot_v3`, which is the CLEANEST sheet measured (margin 20) — so
name which sheet a given character actually draws before calling any of this
player-visible.

⛔ **why nothing caught it**: the drawing canvas IS the logical frame, so overflow
is clipped at draw time, before anything downstream can see it. The only
frame-bound assert in the pipeline is the packer's post-trim losslessness check
(`packer.py`), which compares trim geometry against the logical frame — it cannot
see ink that was never drawn. ⇒ **the guard has to be at draw time**, in the
renderer, and the honest form is the one this repo already uses for rosters:
state the invariant once over what discovery finds, not once per sheet.

⚠ **the fix is two separable things and they should not be confused**: (a) a
CHECK that refuses to publish a clipped frame, and (b) re-authoring the 23 sheets
that are already clipped — some of which may want a bigger logical frame rather
than smaller art. (a) is engine work; (b) is art work and partly Jon's call.

⛔⛔ **AND IT ORDERS AGAINST THE SIZING CLUSTER ABOVE.**
[`engine/sprite-renderer.md`](engine/sprite-renderer.md)'s engine-facing
principle is **measure-by-default**: *"the renderer measures each frame's
canonical body/feet geometry … so the gameplay layer reads geometry from data
instead of guessing."* For a clipped sheet that measurement is faithful to art
**that was already cut** — `body_metrics` then describes a truncated silhouette.
⇒ **migrating a clipped body to `BodySource::SpriteAuthored` would bake the cut
into its collision box.** Fix the clipping first for any body in both lists.
⭐ neither of today's two findings is a new principle; they are the same
principle with **two adopters and no enforcement**.
✔✔ **VERIFIED BY A SECOND ROUTE, because the first one had a live alternative
explanation.** `render_sheet`'s `auto_crop` computes the union alpha bbox across
every frame and crops to it, so *"the art touches the frame edge"* could mean the
frame was fitted to the art rather than the art being cut. The discriminator is
the **opaque-width profile of the topmost rows** — a real tip tapers up from
nothing; a truncated shape starts wide:

```text
super_sanic  idle     top rows ->  12 14 17 18 20 22 24 25   ⛔ no tip: CUT
sanic        idle     top rows ->   0  0  0  0  3  6  8 11   ✔ a taper
sanic        jump     top rows ->   0  0  0  0  0  0  0  0   ✔ touches, NOT cut
player_robot_v3 idle  top rows ->   0  7  9 11 11 13 13 13   ✔ and off.y=73
```

⇒ the art is drawn into a fixed canvas FIRST (overflow cut there) and auto-crop
then hugs whatever survived — a crop can only hug what was drawn. ⭐ `sanic/jump`
is why the two counts differ: it touches the boundary because another frame in
its row sets the union bbox, and it is **not** cut. The run-based criterion
already rejects it, which is the check discriminating correctly rather than
counting edges. ⚠ the check should still report the profile it saw, not just a
verdict.

- ▢ **D128 — Can this engine carry a serious platform fighter through ORDINARY authoring? (product-pressure vertical slice, opened 2026-08-15; FIRST PROOF LANDED)**

⭐⭐ **THE OUTCOME IS PINNED, NOT JUST THE MECHANISM — checked 2026-08-17, and
that is the distinction this row kept blurring.** The 05:45 capture shows both
fighters at **180% and 124% with 3/3 stocks**, i.e. nobody had lost one, which
reads like "launches do not kill". It is not:

```text
the_stage_kills (17 tests, all green)
  a_launched_fighter_is_taken_by_the_world_and_spends_a_stock
  a_second_match_on_the_same_stage_counts_in_and_ends   ← CPU vs CPU
  the_worlds_edge_sits_within_a_launch_of_the_platform  ← a RATIO, not a distance
```

⭐ the second one is the one that matters, and its own comment says why: *"CPU vs
CPU, which is Jon's repro, and ONE stock so the end arrives from a single launch
rather than from minutes of fighting."* So a **CPU-produced** launch reaches the
blast zone and spends a stock — the end is not manufactured by a test writing a
velocity, which is the failure mode this repo has been bitten by before.

⇒ **so the capture is a mid-match MOMENT, not evidence of a broken KO**, and what
is actually open here is **PACING**: how long three stocks take when neither CPU
closes. That is a tuning question for Jon and nothing else — ⛔ do not re-open it
as a defect on the strength of a screenshot showing a high percent.

⚠⚠ **THREE OF THIS ROW'S "NEXT" CLAIMS WENT STALE ON 2026-08-16 — read this
before working any of them.** Everything below is still accurate about the
FIGHTERS; what changed is everything around them.

```text
"THE VFX/SFX ROAD IS BUILT AND UNREACHABLE"   -> REACHABLE (D128 slice, ebc8877ee)
   an effect is a NAME now; ExplosionKind and its three reconstruction tables are
   deleted; GameAssets.fx is an engine-owned home. 189 rows <-> 189 cues, and the
   generic sheets are registered by the ENGINE rather than by Ambition's intro.

"the visual fixes ship UNSEEN"                -> SEEN (D130, de0f25373)
   capture_scene grew `--press touch:XxY`; the two-CPU match is photographable.
   ⭐ and the first look found a real defect nobody had seen.

"does a watcher SEE the two kits behave differently?"  -> ⛔ ASK IT AGAIN, the
   previous answer was measured against a BROKEN MATCH. D131: mary_o and sanic
   author `max_health: 1`, so one point of damage read as a full meter and
   Mary-O hit 4200% in six seconds. The stage now declares the pool every
   fighter's percent is read against (77f246821). **Nobody has watched a match
   since that fix** — so the standing product question is genuinely OPEN again,
   and now askable for the first time.
```

⭐⭐ **RECONCILED 2026-08-17, AND THE PRODUCT QUESTION MOVED AGAIN — HARDER THAN
THE POOL FIX DID.** Any judgement anyone has ever formed about how these
fighters FEEL predates working knockback, and is therefore void:

```text
D155  NOBODY GOT LAUNCHED, and it was two bugs on the shared floor.
      Every authored launch direction in the game was vertically INVERTED
      (an up-tilt drove its victim into the floor at 5048 px/s), and a
      tumbling launch was resolved as a LANDING on the tick it was applied,
      zeroing the velocity. A fighter at 1427% moved zero pixels.
      ⇒ every previous "watch a match" observation was made on a build where
        the percent meter reached nothing and nobody was ever sent anywhere.

D114  hitlag reached only the AVATAR road, so a hit between two ACTORS froze
      NEITHER of them — i.e. every CPU-versus-CPU exchange, which is what a
      watcher watches. Connects had no impact pause at all.

D156  the Patent Clerk and Carl Stargan both RENDERED FACING BACKWARDS: the
      facing was authored three times over and read by nothing.

D146  dash left the vocabulary, shield became a real action, and the smash pad
      got its own profile — so the thing a watcher drives changed too.
```

✔ **THE CAPTURE IS DONE — 2026-08-17, three frames of one two-CPU match
(George Booul vs Pirate Admiral), the documented tap sequence, 1280x720.**
Pixels verified (11,900 distinct colours), so this is a look and not a blank.

```text
  420 ticks   George 34%  · 3/3     Pirate 36%  · 3/3    both fighting
 1200 ticks   George 180% · 3/3     Pirate 124% · 3/3    still nobody dead
 2400 ticks   back at CHOOSE YOUR FIGHTER, every slot NOT PLAYING
```

⭐ **the stock loop CLOSES.** The match ran to completion and returned to the
select screen on its own — which is the thing D140 was about and it holds under
a real CPU match rather than a fixture.
⭐ **the percent meter climbs and the fighters engage** — 34%→180% in thirteen
seconds, with hit VFX on the stage. After D155 that is the first time this has
been true.
⚠ **but at 1200 ticks NEITHER fighter had lost a stock at 180%/124%.** Not
called a defect here — a stock did fall before 2400 — but *"how long a stock
takes at these numbers"* is now an answerable tuning question for the first
time, and 180% with all three stocks intact is worth Jon's eye.

▢ **AND THE CAPTURE FOUND ONE REAL PRESENTATION DEFECT: speech bubbles STACK
ILLEGIBLY.** The 1200-tick frame has three lines drawn over one another —
*"Either you are on the stage or you are not."* twice, and *"Belay that, ye
barnacle!"* printed ON TOP of another bubble's text. Two CPUs taunting at once
is the ordinary case on this stage, so the stack offsets
(`SPEECH_BUBBLE_STACK_STEP` / `_MAX` / `_SPEED` in `ambition_render::fx`) are
not separating what a two-fighter match actually produces.

⛔ **and one thing that is NOT a defect, checked before reporting it**: George
Booul renders as a white GHOST, which reads as a missing texture on the stage.
The select screen's own portrait grid shows the same ghost — it is his authored
art. ⚠ the frame that made it look broken was the stage frame; the frame that
settled it was the roster.
```text
```

⇒ **the cheap next step for this row is one capture of a two-CPU match**, using
the nine-tap command in `capture_scene`'s header, to answer the question the row
was opened to ask. ⛔ do not re-derive the tooling or re-measure the repertoire
first — both are done.

✔✔ **DONE 2026-08-16, AND SOMEBODY HAS FINALLY WATCHED A MATCH.** Twenty-one
frames across two matches through the shipped shell, `--route smash_select --include-ui` plus the
documented nine taps, `--warmup` swept 240/300/330/345/360/375/390/405/420/600/
900/1400/2400 (a frame count is match time, because the press sequence restarts
the capture clock). ⭐ **the answer is split, and the split is the finding: the
two AUTHORED kits do read as two different fighters, and the MATCH is not a
fight — every stock in both matches was lost to the void at ≤6% damage.**

⛔⛔ **AND THE DOCUMENTED NINE TAPS SEAT THE WRONG PAIR.** `747x121` is grid cell
3 and `425x121` is cell 0 — **Sanic and Player Robot v3, the two fighters on the
generic `smash_fighter_kit()` floor**. George Booul is cell 1 (`touch:532x121`)
and the Pirate Admiral cell 4 (`touch:855x121`) in the host's 5x3 grid. So the
command this row points at to ask *"do the two kits behave differently"* seats
the two bodies that have no authored kit at all. ⇒ **fix the two literals in
`capture_scene`'s header and in
`the_capture_tools_documented_taps_seat_two_cpus_on_two_fighters`** (~15 min);
every future look through the documented command otherwise answers the standing
question with the wrong pair. ⚠ the doc block already warns these two points rot
— it is the ROSTER ORDER that moved, not the layout.

```text
GENERIC PAIR (documented taps) — Sanic vs Player Robot v3
 f240  4.0s  2% / 0%   3-3   both bodies STACKED at one spawn point, "GO!"
 f300  5.0s  5% / 0%   3-3   trading at centre-left
 f330  5.5s  6% / 0%   3-3   BOTH off the left ledge; robot drawn PAST the
                             left screen edge; camera still framing the platform
 f345  5.75s 6% / 0%   3-3   Sanic falling, drawn BEHIND the virtual joystick
 f360  6.0s  6% / 0%   3-3   ⛔ EMPTY STAGE — the KO happens off-camera
 f375  6.25s 0% / 0%   2-2   both respawn STACKED again, mid-air
 f600 10.0s  0% / 0%   1-1   two stocks each, gone, to nothing
 f900 15.0s  4% / 0%   1-1   idle, ~300px apart
f1400 23.3s  "seat 2 wins" — Player Robot v3 survives at 0%, 1/3
f2400 40.0s  back on the select screen, all four cards NOT PLAYING

AUTHORED PAIR (portraits swapped to cells 1 and 4) — George vs the Admiral
 f240  4.0s 29% / 36%  3-3   ⭐ 36% traded in FOUR SECONDS; two visibly
                             different kits; both fighters bark
 f360  6.0s 34% / 36%  3-3   the Admiral's anchor/wheel FX plays, ~250px across
 f480  8.0s  0% / 0%   2-2   double KO between 6s and 8s
 f600 10.0s  0% / 11%  2-2
 f900 15.0s  "seat 1 wins" — George survives at 0%, 1/3
```

⭐ **the percents are SANE and D131's fix holds**: nothing above 36% anywhere in
either match, and `--combat-overlay` reads **100/100 over both fighters**. The
4200% meter is gone.

⚠ **what is wrong, in the order I would spend on it:**

1. ⛔⛔ **EVERY STOCK IS A SELF-KO.** Five of six stocks in the generic match and
   all five in the authored one were spent at ≤6%; the winner of BOTH matches
   finished at **0% having taken zero damage all match**. This is not new
   behaviour — `ladder_probe`'s own header already measured "5.0s / 9.8s to
   first self-KO" and noted every level lost all three stocks that way — but
   it is the first time it has been seen as *the whole product experience*.
   ⛔⛔ **THE CAUSE THIS ROW USED TO NAME IS RETRACTED, 2026-08-16 — do not work
   it.** It said `RecoveryPolicy::DRIFT_AND_JUMP` cannot see the authored ledge
   grab. `crates/ambition_characters/src/brain/fighter/recovery.rs`'s own header
   says the opposite in two places: a body whose repertoire commands a
   displacement hands it to the probe as a `RecoveryLift` and **the policy
   becomes `drift+jump+burst`**, and `RecoveryLens::best_route` SEARCHES the
   routes the body owns rather than ranking one statically. Ledge acquisition
   already lives in the real movement kernel, `holds_a_ledge()` already counts
   as recovered, and falling auto-snap already exists. ⛔ **so a second
   ledge-grab model inside `RecoveryPolicy` is the wrong fix and is banned
   here** — it would duplicate the kernel.
   ⇒ **the executable next step is INSTRUMENTATION, not a fix**: take ONE real
   CPU offstage sequence and record, at each frame from the launch to the death,
   `position/velocity → Situation classification → body capabilities →
   perceived terrain → candidate routes → RecoveryOutlook per candidate →
   selected action → executed action → ledge acquisition attempt/result →
   recovered or dead`. The deliverable is **which semantic boundary is wrong**,
   named from that trace. Candidates worth distinguishing, none assumed:
   `Situation::Recovery` entered too late · perceived terrain omits the ledge ·
   probe's initial state differs from the real actor · search predicts a success
   the runtime diverges from · the plan is re-chosen every frame and never
   executed · the stage is genuinely outside the envelope · acquisition
   tolerances · candidate generation excludes a useful tool. Fix the smallest
   real cause, then re-measure with matches, not with a unit test. ⇒ still the
   largest slice here.
2. ⛔ **THE CAMERA DOES NOT FOLLOW A FIGHTER OFF THE STAGE.** f330 draws the
   robot past the left screen edge, f345 draws Sanic behind the touch stick,
   f360 is an EMPTY STAGE, and expr f360 clips George's nameplate at x=0. The
   one moment a platform fighter must show is the one it never shows. ⇒ the
   framing policy exists (`CameraSnapshot2d`); it needs *frame every live seat*
   rather than one focus. Half a day, and it makes defect 1 legible.
3. ⛔ **BOTH SEATS SPAWN AND RESPAWN AT ONE POINT, OVERLAPPING** (f240, f375,
   f420) — `respawn_placement(stage_centre())` for both. Same shared
   `ActorConfig.spawn.pos` this row already blames for the bit-symmetric brains.
   ⇒ an offset by seat index plus a test, an hour.
4. ⚠ **THE WINNER CARD NAMES A SEAT** — "seat 2 wins" / "seat 1 wins", never the
   fighter — and there is NO results screen: by f2400 the shell is back on the
   lobby with all four cards cleared, so a couch rematch re-seats everybody.
   ⇒ `victory_banner` wants a display name, an hour; the rematch flow is a
   product decision.
5. ⚠ **HIT BARKS DRAW AS A SCREEN-WIDE CAPTION ACROSS THE PLAY AREA.** expr f240
   renders "GO!" and *"Either you are on the stage or you are not."* on the SAME
   LINE, so the countdown is illegible; f360/f600 keep a full-width quote beside
   the action. ⭐ they are the combat hit barks (`ambition_content::banter`) and
   the catalog `fallback_dialogue` — so their presence is real evidence hits are
   landing, and their placement is the bug. ⇒ scale + placement, hours.
6. ⚠ **AN UNTEXTURED BODY-SIZED QUAD** (~64x85px at 1280x720, olive) is drawn
   beside Player Robot v3 during exchanges (f240, f300). ⛔ NOT a combat volume —
   `--combat-overlay` outlines the real hit/hurt boxes and leaves this one
   unoutlined — and NOT a named actor (`--dev-overlays` gives it no nameplate).
   Absent from the authored-kit match, which draws real FX art. Suspect the
   "bare colored rectangle (no entity sprite available, no atlas)" fallback in
   `ambition_render`'s `rendering/actors/mod.rs` (~line 597), i.e. something in
   the generic kit's effect path binds no sprite. ⇒ an hour to identify.
7. ⚠ **VFX SCALE**: the Admiral's wheel/anchor effect is ~250px across against a
   ~45px fighter (expr f360) and occludes both the fighter and the stage. The
   art road works — the sizing is authored against nothing.
8. ⚠ **`capture_scene` prints no pose for the state it exists to photograph**:
   its `subject at (x,y)` line is `PrimaryPlayerOnly`, and a two-CPU match has no
   primary player, so every log above is silent about where anybody was. ⇒ print
   each `MatchSeat` body, ~20 lines.

⇒ **the plain answer to the standing question:** *not yet, and the reason is no
longer the fighters.* With George Booul and the Pirate Admiral seated, a watcher
sees two mechanically different bodies inside four seconds — different
silhouettes, different effects, a cutlass against a Boolean ghost, 36% traded —
so **"content underuse" is answered: the authored kits DO read**. What the same
watcher does not see is a fight: the match is over in 13-23 seconds, every stock
is lost to the void at nearly zero percent, the camera is pointing at an empty
platform when it happens, and the game announces *"seat 2 wins"*. ⛔ **the next
spend is the stage-return loop (1 + 2), not more repertoire** — a fighter that
cannot get back on the stage has no room to show a repertoire at all.

✔✔ **FIRST FIGHTER LANDED AND VERIFIED (George Booul, 2026-08-15)** — Smash lib
73, Smash app 21, characters 531, core 393, app_it 365, gate clean.

⭐⭐ **and it answered the question it was sent to answer: MOST OF THE GAP WAS
AUTHORED REPERTOIRE, NOT MISSING ENGINE.** The `special_*` verb chain,
`Cancelable`/`OnHit`, per-volume `on_hit`, `motion_scale` tails, multiple Active
windows and per-move Sfx/Vfx were **all already sufficient and adopted by
nobody**. Exactly one capability was genuinely missing: **a move could not
displace its owner at a chosen moment, nor command a speed rather than add to
one** ⇒ `MoveFrameData::{lift_speed, lift_at_s}`, which the recovery probe now
presses as a `RecoveryLift`.

⚠ **two gaps NAMED rather than half-built**, and that judgement stands: an
airborne self-impulse move has **no per-airtime budget** (a use counter is
rollback state and the schema was not that pass's to re-baseline), and
`WindowTag::Invuln`/`Armor` are declared vocabulary with **no consumer** —
authoring them parses and does nothing.

✔✔ **SECOND FIGHTER LANDED TOO (Pirate Admiral, 2026-08-15)** — a materially
different lateral/grapple recovery concept, so the recovery ontology is no longer
a single positive vertical `Set` impulse. Recovery routes are now evaluated
through the **real movement kernel** (`movement/recovery.rs` drives a scratch
body) rather than ranked by a static "this is the recovery move" property.
⛔ **do not redispatch "make a second fighter".**

⇒ **NEXT is a PRODUCT question, not a roster question:** does a watcher actually
*see* the two kits behave differently? The behavioural claims owed are
`Recovery situation → useful authored recovery action selected → route has a
meaningful chance to regain the stage`, and `two different authored kits →
observably different fighting behavior`. ⛔ *"the special exists"* and *"the
special occurred somewhere in 1800 ticks"* are both rejected as evidence.

✔✔ **THE CPU LANE LANDED AND WAS MEASURED (2026-08-15).** Distinct attacks used
per match **5-6 of 16 → 9**; all four of George's specials now appear;
`modus_ponens` was **selected 19-24 times a match and performed ZERO**. The
duelist's whole vertical game (`air_up`, `air_down`, `smash_up`, `tilt_up`) was
absent and is now present.

⭐⭐ **the two causes, and BOTH of my stated hypotheses were wrong:**
1. **the brain could not AIM.** The attack stick wrote a facing-relative `+x`
   into a field the resolver multiplies by facing, so every forward/back attack
   chosen while facing left came out reversed — and it shoved to full deflection,
   which the gesture resolver reads as a FLICK, so **the brain could not ask for
   a tilt at all**. ⛔ that, not seat-keyed input, was the mirror asymmetry.
2. **the `.first()` fallback was worse than its doc warned.** The kernel's route
   search endorsed a recovery in **3 of 100** `Situation::Recovery` decisions;
   the other 97 pressed the Up-B anyway. Deleted outright — a search that
   endorses nothing now presses nothing. ⚠ `least_bad_route` was proposed and
   REJECTED after a grid over the real stage: there is nothing to rank, because
   George's `Set (0, -1020)` erases the drift that would carry him back.

✔ **Jon's three couch items, all fixed:** the camera close was **237-361 units in
ONE frame** against a 33-49/frame open ramp (now eased, 68.9); the match-end race
is Smash despawning the eliminated body while `decide_stocks_match` reads sides
off bodies that still exist, with nothing ordering them; and the countdown **was
running** — it announced into `GameplayBannerRequested`, which **nothing in the
workspace draws**. ⛔ the winner card had the identical defect and its unit test
was green throughout, because it asserted the message.

⇒ ⭐⭐ **NEXT, and it is the highest-value thing left in this lane: THE VFX/SFX
ROAD IS BUILT AND UNREACHABLE.** **166 `vfx.*` cues ship in `sfx.bank`**,
including complete per-move sets for both expressive fighters
(`vfx.george_booul.up_b.{windup,launch,ascent,tail}`, `modus_ponens_dash/impact`,
`reductio_drop/bounce/impact`; `vfx.pirate_admiral.grapple_cast/catch/tension`,
`heave_to_anchor/brake`, `cutlass_wake/clash`) — and **ZERO are referenced from
any Rust file**. George's four specials all play the generic robot slash.
⛔⛔ **and the new FX spritesheets are unreachable BY DESIGN**: `move_vfx_kind`
maps only five effect names and `MoveSpec::presentation_problems` **REFUSES**
anything else at startup, so an authored move cannot name the new art even after
the sheets are registered.

✔✔ **LANDED 2026-08-16 (`ebc8877ee`): AN EFFECT IS A NAME, AND THE ENGINE SHIPS
THE ART IT DRAWS.** `FxId` is the authored row name on the wire — `SfxId`'s own
FNV-1a hash, *borrowed rather than re-typed*, because the two id spaces name the
two halves of one authored thing and a second copy is a place for them to
disagree. `Copy`, 8 bytes, so a message allocates nothing at RL rollout rates.
`ambition_sprite_sheet::fx` resolves name → (sheet, row via `first_bound_row`,
`vfx.<family>.<row>` cue), and `GameAssets.fx` is the home that did not exist,
filled by the engine's own `load_game_assets` — an FX sheet is neither a
character nor an LDtk prop, and it no longer squats in a map keyed by
`Prop.kind`. **FIVE tables deleted, not four**: the fourth-and-a-half was five
aliases in `CharacterAnim::from_name` spelling effect rows *Idle/Walk/Run/Hit/
Slash*. 374/0 app, 28/0 smash app, 29/29 contracts.

⭐⭐⭐ **AND THE "ONE REAL DESIGN CONSTRAINT" BELOW DISSOLVED RATHER THAN BEING
ANSWERED — which is the reusable lesson.** The row said the validator runs at
roster install with no world and no loaded assets, so the vocabulary could not be
read off the sheets, and offered two options: a declared 189-row table pinned by
a both-ways test, or dropping the refusal for SFX's open-vocabulary policy.
**Neither was needed.** `build.rs` already bakes every `*_spritesheet.ron` into
the binary, so the art itself is a pure, world-free oracle
(`ambition_sprite_sheet::fx::is_authored_effect`) — and `expand` takes it as a
PARAMETER rather than naming that crate, so a headless RL build still does not
link an image-decoding presentation crate. ⇒ ⛔ **"no world at validation time"
was a constraint on ASKING A RUNNING APP, not on knowing the data.** The two
shipped moveset tests now validate against the real 189-name oracle instead of a
five-name enum, so the guard got stronger while the table disappeared.

⇒ ⛔⛔ **NEXT, and it is the same defect one level up: THE STANDALONE SMASH APP
COMPOSES NO ASSET INSTALL AT ALL.** `GameAssets` is inserted by exactly one
plugin (`PlatformerAssetsPlugin`), `game/ambition_demo_smash_app` installs
engine + host + debug-viz and *not* that, so the resource never exists in that
process and nothing sheet-driven has art there. Adding the plugin panics:
`bind_game_assets` takes `AuthoredSheets` and `BossCatalog` as hard `Res`, which
that composition never registers — **art the engine knows how to draw, with
nothing in the composition to hand it over.** ⚠ **scope it correctly before
acting**: `ambition_app` loads assets in its own setup, so *Smash reached through
the shell is a different composition from the standalone binary*, and only the
standalone one is bare. Say which binary any claim is about.
⚠ smaller, same family: the FX sheets load via
`asset_server.load("<sprite_folder>/<target>_spritesheet.png")` outside the asset
manifest (Sanic's ring set the precedent) — fine on desktop, and the known
Android-packaging blind spot.

⭐⭐ **MEASURED 2026-08-16, and it reordered the work — the refusal was the THIRD
edit, not the first.** Three findings, each read off the shipped data rather than
inferred:

1. ⛔⛔ **THE SHEETS ARE ABSENT FROM EVERY DEMO, NOT MERELY LIMITED.** The FX
   sheets are registered by exactly one table — Ambition's intro
   (`game/ambition_content/src/intro/sprites.rs`) — into
   `GameAssets.characters.props`, *a map keyed by the LDtk `Prop.kind` field*.
   The Smash/Sanic/Mary-O apps register character sheets only, so
   `spawn_explosion` takes its `else` branch (a particle burst) **every time**.
   ⇒ engine-level FX-sheet registration is the PREREQUISITE; the vocabulary
   widening buys little until it lands, because there is no art to name.
2. ⭐⭐ **THE ART AND THE AUDIO ALREADY AGREE NAME FOR NAME.** Four generic
   sheets ship 65 rows — `generic_action_fx` 18, `generic_world_fx` 18,
   `generic_exotic_fx` 24, `generic_explosions` 5 — and `sfx.bank` ships
   `vfx.<family>.<row>` for **every one of them** (`dash_streak`, `ice_shatter`,
   `sonic_boom`, `rune_burst`, …). One authored name can therefore yield both
   the clip and its paired cue with no third table.
3. ⛔ **so `ExplosionKind` is a hand-kept transliteration of a naming the data
   already carries twice.** Its 5 variants ARE the 5 rows of
   `generic_explosions`, and three tables reconstruct that: `move_vfx_kind`
   (name→enum), `explosion_anim` (enum→`CharacterAnim`, i.e. FX rows addressed
   as *Idle/Walk/Run/Hit/Slash*), `explosion_sfx` (enum→cue). ⇒ the deletion is
   those three, and the seam to delete them into **already exists**:
   `SheetRecord::first_bound_row(chain)` was built 2026-08-11 as *"the seam that
   lets an authored CLIP be drawn without an engine enum variant"*.

⚠ **the one real design constraint**: `presentation_problems`' oracle is already
INJECTED (`prefab_registry.rs` passes `|id| move_vfx_kind(id).is_some()`), but it
runs at **roster install** — a pure function with no Bevy world and no loaded
assets. So a widened vocabulary cannot be read off the sheets at validation time;
it needs a declared table in the `sfx_ids!` shape (one declaration emitting both
the constants and the name list) **pinned by a test that reads the shipped
`_spritesheet.ron` rows and asserts set equality BOTH ways**, or the refusal has
to be dropped in favour of SFX's own policy (open vocabulary, a counted miss).
⚠ also: **no SFX bank is resident in the demo app at all**, so the paired cue
half is a second registration gap with the same shape as (1).

⭐ **the CONTENT half of this road advanced 2026-08-16** — the maintainer's four
VFX-authoring commits in each renderer submodule (George Booul's Boolean ghosts,
the pirate and ninja leaders', Oiler's, each with its procedural SFX companion)
are now consumed by the superproject (`989dc3318`). Checking that pointer
advance against *"does the root actually consume it?"* found the pipeline gap
that shape always hides: `george_booul_vfx` and `oiler_vfx` were authored in the
submodule and named in **no line of `regen_sprites.sh`**. George's sheet is
published only because someone ran a focused `--target`; a fresh clone's regen
would have dropped it. Roster fixed and the invariant now stated over discovery
rather than once per sheet (`b2e3eeafe`).

✔✔ **AND IT ALL GENERATES NOW (Jon asked, 2026-08-16).** `./regen_sfx.sh`
rendered **38 missing cues** and repacked (bank 166 → **189** `vfx.*`);
`./regen_sprites.sh --target george_booul_vfx --target oiler_vfx` published both
sheets including the reduced-resolution tiers. ⚠ none of it is in git — the
audio/sprite assets are gitignored, so **the roster commit IS the durable half**
and a fresh clone gets these only by running the two scripts.

⭐⭐⭐ **AND THE MEASUREMENT IS NOW EXACT: 189 ROWS ↔ 189 CUES, ONE FOR ONE,
ACROSS ALL TWELVE SHEETS.** 4 generic sheets = 65 rows, 8 character sheets = 124
rows (`carl_stargan` 12, `george_booul` 21, `ninja_shadow_oni_leader` 14,
`noether` 12, `oiler` 23, `patent_clerk` 14, `pca` 14, `pirate_admiral` 14) — and
`sfx.bank` carries exactly that many cues, family by family, with **no sheet off
by one**. ⇒ the unit of this vocabulary is **the effect NAME**: it already
addresses the art and the sound together, in the data, with no Rust in the
middle. The engine owes ONE mapping (name → which sheet holds the row), not the
three tables `ExplosionKind` currently needs.

⇒ **the design, the deletion gate and the Enoki bearing live in
[`engine/render-animation-and-vfx-extension.md`](engine/render-animation-and-vfx-extension.md)
§ "MEASURED 2026-08-16"** — that plan already owned this ground (its VFX-08 names
the sibling defect), so the row points rather than restates. ⛔ the deletion gate
is `ExplosionKind` + `move_vfx_kind` + `explosion_anim` + `explosion_sfx`; a
slice that adds a new message beside `VfxMessage::Explosion` has wrapped the old
model, not removed it.

✔✔ **LANDED 2026-08-16.** All four deleted, plus a FIFTH table nobody had
counted: the `classic_burst`→*Idle* / `burst_round`→*Walk* / `shockwave`→*Run* /
`smoke_burst`→*Hit* / `starburst`→*Slash* aliases inside
`CharacterAnim::from_name`, which existed only so the explosion sheet could be
loaded through the character path. An effect is an `FxId` (FNV-1a, `SfxId`'s
shape) resolved against `ambition_sprite_sheet::fx`, which walks the twelve
declared FX sheets' BAKED records — so name→(sheet,row,cue) is derived, not
declared, and 189 rows are reachable. `GameAssets.fx` is the engine's own sheet
slot and `load_game_assets` fills it; the intro's LDtk-prop row for
`generic_explosions` is deleted. ⭐ **the "one real design constraint" dissolved**:
`build.rs` already embeds every `*_spritesheet.ron`, so the vocabulary IS
readable at validation time with no App and no loaded assets — no declared table,
no dropped refusal. `MovePrefabRegistry::expand` takes the oracle as a parameter
rather than naming a crate a headless RL build must not link.
⛔⛔ **and it exposed the next one: the Smash shell installs no
`PlatformerAssetsPlugin` at all** (Mary-O, Sanic and Twintrack each do), so
`GameAssets` does not exist in that process and NOTHING sheet-driven has art
there — fighters included. Adding the umbrella install panics: `bind_game_assets`
demands `AuthoredSheets` + `BossCatalog` as hard `Res`, which that composition
never registers. Same defect shape one level up.

⚠ **remaining showcase weaknesses, in order:**
- **the mirror match is bit-symmetric.** Brains seed from the level alone
  (`0x5F37_7A11 * (level+1)`) and the comment approves. A per-body seed was tried
  and **reverted** — `ActorConfig.spawn.pos` is shared between seats at the
  construction site, so it differentiated nothing.
- **the recovery search cannot see the LEDGE GRAB.** These fighters author
  `ledge_grab: true`, the engine implements grab/hang/climb/getup fully, and
  `RecoveryPolicy::DRIFT_AND_JUMP` never presses toward it — so a body beside the
  lip is reported unrecoverable where a player would catch it.
- **the two expressive fighters never meet** — the instrument lives in the demo,
  the Admiral in `ambition_content`. ⭐ provider-composition evidence, correctly
  reported rather than solved by violating the demo gate.
- ⚠ **the repertoire histograms are single samples**: `build_demo_app` does not
  pin `TimeUpdateStrategy` the way `smash_in_the_host` does, and two full-file
  runs gave different hashes. Directions of change are far larger than the
  variance; the exact counts are not reproducible yet.

⭐ **the question, and it is a product question rather than an architecture one:**

> can someone watch a CPU-vs-CPU match and immediately see several mechanically
> distinct attacks, aerial choices, specials, an intentional recovery move,
> expressive movement and convincing impact — and conclude this engine can
> elegantly support a serious platform fighter?

⚠ **the pressure is real and current:** CPU-vs-CPU matches already run and are
nontrivial, which is encouraging, but they **visually undersell the engine
badly**. Characters mostly double-jump, throw generic hitboxes, use legacy
dash/blink as recovery, and lack convincing Up-B, distinctive specials,
tilt/aerial identity and audiovisual impact.

⭐⭐ **the FIRST deliverable is a distinction, not a feature:** which of that is
**content underuse** and which is **genuinely missing engine capability**?
Inspection suggests the moveset runtime already supports considerably more than
the content exercises. ⛔ **only the second justifies engine work.**

**Scope:** one existing body, ≥8 materially distinct attacks (⛔ rotated clones do
not count), a real authored **Up-B** in the ordinary moveset architecture, a
launcher, a punish/kill move, a mechanically interesting aerial, and authored
SFX/VFX through normal content mechanisms.

⛔⛔ **CPU usage is part of acceptance, and it is where this gets architectural.**
The generic policy layer must actually use the repertoire — ≥5 distinct offensive
move ids, aerials, a special, and the authored Up-B for recovery. ⛔ **no
character-ID conditionals in AI.** ⭐ derive affordances from move data
(coverage, startup, reach, launch direction, commitment, impulse) before adding
annotations; only a technique whose behaviour cannot be inferred from static
geometry — teleportation is the example — may expose its own affordance.

⛔ **not authorized:** grab/throw architecture · shields/parries as a subsystem ·
ledge-rule parity · many characters · balance · animation redesign · combo
scripting · networking. ⛔ and no character-specific system to compensate for a
missing generic mechanic.

⚠ **a real customer for reachability, and the only sanctioned link to it:** an
authored Up-B is exactly what `RecoveryPolicy` should be able to consider — its
default presses only `side ∈ {0,±1}` plus jump. ⛔ do **not** begin the general
navigation graph.

⇒ **this is why the run temporarily carries THREE workers** (maintainer's
exception): the combat lane is orthogonal enough to the systemic-world and
rollback lanes to stay independently integrable. ⛔ narrow or pause it the moment
it starts changing the same authority boundary as another live lane.

## Standing continuation rule

**This file is a continuation LEDGER, not a terminal checklist.** There is no
"the queue is empty, therefore stop" state: an empty executable list is a signal
to re-measure HEAD and refill, never a completion condition.

When the executable rows above close, **do not stop**. Re-read HEAD and promote
the next highest-value verified card from [`tracks.md`](tracks.md), a new direct
maintainer direction, or a reproducible maintainer observation.

Prefer in order:

1. Ambition flagship needs that create reusable engine capability;
2. Engine-1.0 ownership/composition/authoring work;
3. serious secondary game/acceptance pressure such as Smash or TwinTrack; and
4. deferred/trigger-based work only when its trigger is present.

Do not add meta-work merely to keep the queue nonempty. The queue continues by
finding real product or architecture work, not by manufacturing process.
