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

⛔⛔ **A CLOSED ROW IS A RECEIPT, NOT A CASE FILE — and this file learned that
the expensive way.** On 2026-08-17 **2,584 of its 7,025 lines were closed rows**,
each carrying the full investigation that justified a fix nobody was going to
revisit. The form a closed row takes, and the whole of it:

> `✔ **D123 — what was wrong, in one sentence.** Fixed by `<commit>`: what the
> fix was. Guarded by `<test>`. ⛔ <a standing prohibition, only if one exists>.`

⭐ **the evidence lives in the commit message and in this file's own git
history** — `git log -p docs/planning/queue.md` recovers every word of it. ⚠ the
same rule applies INSIDE an open row: keep the current model at the top and
delete the layers it supersedes, because a stale `⇒ NEXT` sentence under a
correction is exactly how a later session re-does landed work. The narrow
exception is a sentence that would otherwise be **rediscovered at cost** — a
prohibition, an instructively wrong measurement, a design refused for cause —
and that is one clause, never a section. Full rule in
[`README.md`](README.md#queue-contract).

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

⭐ **REFILLED 2026-08-17**, following this table's own rule that it must never
describe the last run. D125's lane CLOSED (the start-room seam landed and all 24
callers migrated). D33 ran three slices and is parked with the monolith **under**
its frozen baseline.

| Lane | Owner | Executable next action |
|---|---|---|
| **D127 — authored logic** | **unstaffed** | ⛔ M1 is complete and M2's prepared-call half LANDED (`7e7552c4b`); the `when … then` rule form is deliberately UNBUILT for want of a customer. ⇒ nothing here is dispatchable until a customer appears or M5 diagnostics are wanted — **do not re-open M1 or M2** |
| **D128 — Smash CPU showcase** | unstaffed | ◐ **ENGINEERING IS DONE — every line closed by 2026-08-18.** Pacing ACCEPTED (Jon, 2026-08-17: under ~40s is *"if anything… brisk"*) ⇒ ⛔ do NOT retune stock count, knockback or damage. Respawn placement, standalone asset composition, CPU symmetry and all four presentation defects are ✔ — the last two were the bark width and the untextured impact quad, both photographed before and after. ⇒ **what remains is Jon watching one match**, not another capture. |

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

⭐⭐ **THIRTEEN ENTRIES IN THAT FILE WERE RULED ON 2026-08-17**, so re-read it
before promoting from this table — several rows here are answered. Highlights:
`F` still opens doors (DRIVEN headlessly, with the repro recorded); the title's
60 FPS is not intentional; the quality-change character swap has three causes
eliminated and the right road named; the stray sword, the bolt hurtbox and the
Mary-O restart are all **decisions**, not defects, and are now cross-linked to
`awaiting-maintainer-decision.md` — which is where their options live.
⛔ **two of my own rulings that day were WRONG and are marked as withdrawn in
place**: couch play is NOT switched off, and a clipped label is not a defect.

| Observation | Why it is worth a lane |
|---|---|
| Super Sanic's spikes are clipped by the sprite renderer | ⭐ Jon called it structural himself — *"we should not be able to clip sprite artwork so easily"*. This is the only one that is an ENGINE gap rather than content |
| Mary-O secret/invisible blocks keep their brick texture when spent (quasar brick in 1-1) | the spent-art road already works for `?`-blocks, so this is a road that skipped a case |
| ~~Mary-O allows one fireball; should allow two~~ | ✔ **ALREADY DONE, ruled 2026-08-17** — `MAX_LIVE_SPARKS` is 2 and the guard counts LIVE SHOTS rather than reading the constant back, so a retune cannot make it vacuous |
| ~~the multi-coin block's coin-pop VFX~~ | ✔ **RESOLVED 2026-08-15 and it was never missing** — it landed in `943a9aa0c`; four demo shells had no `VfxMessage` reader, so it drew in the full game and nowhere else. ⛔ the doc entry said otherwise for a day |
| the snake and AI slop are far too big, and the snake sprite may not match its box | ⚠ related to the player-side sprite/box unit mismatch at the top of that file — the two may be one bug |
| **Sanic is very small in his own game** (Jon, 2026-08-15) | ⭐⭐ **third body in the sprite/box cluster, and the one that makes it a CLUSTER rather than three bugs** — see the measurement below |
| ~~drop `pocket` and `versus` from the main game-selection shell~~ | ✔ **DONE, confirmed by Jon 2026-08-17.** Both call `.unlisted()` (`demo_pocket/src/lib.rs:190`, `app/versus.rs:905`) and `launch_entries()` filters them, so they stay composed, routed and testable while no launcher row advertises them. ⭐ the distinction the flag states: an *unavailable* experience still appears greyed with its reason, because the player is meant to see it exists; UNLISTED is the other case — composed and routed but never for the player to choose |

### ▢ Two things found in passing 2026-08-15, logged rather than fixed

**1. ⚠ TWO WORLDS FAIL LDtk VALIDATION TODAY, and the tool writes them anyway.**

⭐⭐ **MEASURED 2026-08-17 — and it is ONE world, not two. The count depends on
WHICH PATH you validate, which is a footgun worth more than the finding.**

```text
game/ambition_demo_mary_o/assets/worlds/mary_o.ldtk    exit 0 · 0 errors · 5 warnings
game/ambition_map_assets/…/worlds/mary_o.ldtk          exit 1 · 26 errors
game/ambition_content/assets/worlds/sandbox.ldtk       exit 1 · 4 errors · 8 warnings
```

⛔ **the same file, twice.** The demo path is a SYMLINK into `ambition_map_assets`,
and the sidecar manifest a world is validated against —
`<world>.entities.json`, resolved strictly BESIDE the path you name — sits next
to the **symlink**, not next to the real file. ⇒ validating the raw copy invents
**26 `MaryOBlock` errors** that do not exist through the canonical path.
⭐ **Mary-O is CLEAN.** Its manifest declares `MaryOBlock` and always did.

⚠ **so only `sandbox.ldtk` genuinely fails, with 4 errors, and they are false
positives too**: all four are cross-world `LoadingZone` targets
(`intro_wake_room`, `gate_stack_lower`, `hall_of_characters`,
`you_have_to_cut_the_rope`). Those rooms EXIST — this session drove a live
transition through one of these doors — the validator is single-file and cannot
see a sibling world.

▢ **so the fix is cross-world resolution (or a documented suppression) for four
edges, and nothing for Mary-O.** ⛔ do NOT hand-write an entities manifest for the
map_assets copy: that file is *"the same shape `def register-entity --spec`
consumes"*, i.e. what editor definitions are GENERATED from, so a second copy is
a fork of an authoring source.

⚠ **invocation, because getting it wrong looks like success**:
`PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools validate <world>`
— the package is not installed in this environment, so a bare `python3 -m …`
prints *"No module named"* and exits 1, which a naive error-count reads as clean.
⚠ and its diagnostics are INDENTED, so `grep -c '^error:'` returns 0 on a failing
run. ⛔⛔ **the errors do not block the write**, which is how they cost a
correction: three `error:` lines filled a `| head -3` and hid the `wrote` line
under them, so an edit that HAD landed was reported as refused.

⇒ **this block is now the SHORT version; the row that owns the instrument is
D163** (which also carries the retracted "duplicate pirate spawns" finding — ⛔ the
coincident rider/mount pairs are AUTHORED and must not be deduplicated).
⛔ **do not re-derive the counts here**: `MaryOBlock` is a false positive of
validating the raw copy, and `mary_o.ldtk` is CLEAN through the canonical path.

**2. ◐ THE SMASH LANE'S VISUAL FIXES HAVE NOW BEEN PHOTOGRAPHED — what is left is
Jon's eye, not a capture.** ⛔ **stale as written on 2026-08-15 and corrected
2026-08-17**: it said the camera-close ease, the 3-2-1-GO card and the winner card
had *"none been LOOKED at"*. All three have since been captured through the
shipped shell (D130 gave `capture_scene` `--press touch:XxY`; two-CPU matches were
photographed 2026-08-16 and again 2026-08-17), the winner card is verified naming
the fighter, and the countdown is verified drawing. ⇒ **what genuinely remains is
one judgement, not an instrument**: whether 5 Hz is the right close rate — a
number that was chosen, not measured. See D128's ACTIVE TRUTH block for the one
outstanding product-acceptance item; ⛔ do not dispatch another capture to
establish status.

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

- ▢ **D117 — Finish the controlled-character actor kernel. UNBLOCKED 2026-08-17:
  the decision it rested on is ANSWERED.**

⭐⭐ **the blocker is gone.** This row waited on the hit-emphasis / proper-time
question (`awaiting-maintainer-decision.md` **#6**), and Jon ruled it on
2026-08-17: hitlag freezes the BODY that is in it, on both roads. *"Does the
merged integrator freeze an actor body on its own hitstop?"* WAS that question,
and the answer is yes. ⇒ **the movement/TIME integrator fork is executable now**,
as is folding the three per-population `decay_reaction_timers` calls into one
(the controlled site decays on `frame_dt`, the other two on sim `dt`).

⭐ **what is already done, measured against HEAD**: control authority CONVERGED
(one `tick_controlled_brains`), and `tick_actor_brains` reads as a sequence after
three extractions.

✔✔ **THE STEP ITSELF IS ONE SEAM NOW (2026-08-18).** Both roads reached
`ae::step_motion` by writing the same two lines beside their own call — refresh
the axis params from the resolved tuning, then zero `dt` if the body is in
hitlag. **That is exactly how D114 happened**: the freeze was a line one road had
and the other did not, so a hit between two AI bodies froze neither. Those two
steps are now `ambition_characters::actor::step_body`, and neither road spells
them.

```text
before   avatar/body_integration.rs   axis params · hitlag · ae::step_motion
         features/enemies/integration axis params · hitlag · ae::step_motion
after    both                         → actor::step_body(.., combat, tuning, ctx)
```

⭐ **it takes the BODY, not a `dt`.** A `dt` parameter is something a caller can
compute wrongly, and one of them did for months; passing `&BodyCombat` means the
rule is asked, not remembered.

⛔⛔ **THE "FOLD THE THREE `decay_reaction_timers` CALLS" ITEM IS REFUSED WITH
CAUSE — 2026-08-18, and the refusal cost seven boss tests to establish.** They
iterate different populations in different phases, so merging the SYSTEMS would
build a god function; what they fork on is the CLOCK, and that fork is correct:

```text
actor tick    world_time.sim_dt()   scaled — slows with bullet-time
boss tick     world_time.sim_dt()   scaled
controlled    time.delta_secs()     RAW — and this is DELIBERATE
```

⭐⭐ **because HITSTOP IS A `sim_clock` REQUESTER.** A connect asks the sim clock
down, so decaying `hitstop_timer` on `sim_dt()` slows the timer that ENDS the
freeze by the freeze itself, and stretches the i-frame and hitstun windows
measured against the same scale. ⇒ i-frames are a promise to the player in REAL
seconds; a bullet-time moment must not hand out longer invulnerability, which is
the same reason the double-tap windows are unscaled.

⛔⛔ **AND THE `Res<Time>` WAIVER SAID SOMETHING FALSE, WHICH IS HOW THIS
HAPPENED.** It claimed *"the reaction timers still compute their own scaled dt
manually"* — no such scaling exists or should. I read the false sentence, checked
the code, and "corrected" the code to match a rule the sentence implied.
`boss_contact_iframes`, `boss_lifecycle` and `boss_motion_parity` refuted it
within one run. ⇒ ⭐ **a false justification does not mean the decision under it
is false**, and consolidating a fork nobody explained is how a deliberate one
gets undone. The waiver now carries the real reason, and
`the_reaction_timer_clock_forks_on_purpose` pins BOTH sides — a fork guarded on
one side only drifts back.

⭐ **verified NOT a determinism defect on the way**, which was the first
suspicion: `BodyCombat` is rollback-registered and this decay runs in the sim
schedule, but production pins `TimeUpdateStrategy::ManualDuration` to the sim
tick whenever rollback participants exist, so the raw delta was deterministic —
a different clock, not a desync.
⭐ **placement was decided by reading the destination's contract, not by
convenience** — `ambition_characters` says its job is *"the same brain +
control-frame contract drives players, NPCs, enemies, and bosses"*, and
`ae::step_motion` says *"model dispatch happens inside the trusted kernel, while
body/controller identity remains outside"*. A body's hitlag IS body identity, so
core cannot host it and the actor-behaviour crate should. ⛔ it deliberately did
NOT land in the monolith — Jon's standing rule the day before: *"try not to dump
things into it to make the problem worse."*
⚠ guards are a PAIR and the pair is the point:
`a_body_in_hitlag_does_not_travel_through_its_own_freeze` plus
`and_the_same_body_travels_once_the_freeze_clears`, because a body that never
moves passes the first for the wrong reason. Poison-verified — deleting the
branch walks the frozen body **8.98px** and leaves the control green.

✔✔ **AND THE HOME ROAD WAS REBUILDING THE COLLISION WORLD PER BODY.**
`integrate_sim_bodies` composites `world_with_sandbox_solids` once per frame for
the actor loop, and then the home road called it AGAIN, per body, from identical
inputs — cloning the whole block list to rebuild a value that already existed a
few lines up. Both roads take the one composited world now, and
`integrate_home_body` loses three parameters. ⭐ the deeper win is not the clone:
**two composite sites is two places for the moving platforms, gate solids, water
and portal carves a body collides with to drift apart.**

⭐⭐ **AND THE ANSWER TO "SHOULD THE TWO INTEGRATORS BECOME ONE FUNCTION?" IS
NO — MEASURED, NOT ASSUMED.** All four pairs were compared:

```text
the step         ✔ MERGED — actor::step_body, one seam, both roads
footprint        ✔ MERGED — publish_body_footprint, one rule, both roads
input build      DIFFERENT FOR CAUSE — an actor is steered by its brain's
                 velocity_target projected through a flight limb; a home body's
                 axes ARE the stick
reset decision   DIFFERENT FOR CAUSE — home reports a `BodyReset { cause, origin }`
                 that authored `DeathRules` consume; the actor road ticks a
                 RespawnPolicy::InPlace timer and revives itself. Two different
                 questions wearing one word
```

⇒ **fusing the last two would build exactly the god function this milestone
forbids** (*"no replacement god `ActorContext`/service bag"*) — an
`Option`-per-species parameter list whose body is two `if`s. ⛔ so
`integrate_home_body` STAYS, and the deletion of that name was the wrong target:
the two roads live in disjoint queries (`With`/`Without<PlayerEntity>`) with
different cluster shapes and cannot share one Bevy loop anyway.

⭐⭐ **THE PROPERTY THAT ACTUALLY MATTERS IS TRUE NOW, AND IT IS CHECKABLE:
production has ZERO direct `ae::step_motion` calls.** Every body — home, actor,
seated fighter, boss — reaches the movement kernel through `step_body`; the only
two remaining spellings in the monolith are both inside `#[cfg(test)]` helpers.
That is what *"controlled and AI bodies use the same body/control contracts"*
means operationally, and unlike a function name it cannot be satisfied by a
rename.

✔✔ **THE FOOTPRINT IS ONE RULE NOW, AND THE BOUNDARY THAT REFUSED IT WAS
STATING A FALSEHOOD.** `publish_body_footprint` is the single publish; both roads
call it, and the actor road's coarse-envelope override became a PARAMETER rather
than a species.

⛔⛔ **the refusal is the part worth keeping.** `attack_geometry`'s header said
*"this is boss-attack-specific geometry only"*, which reads exactly like the
stated contracts D136 celebrates — and it turned a correct move into an
obviously-wrong one at zero cost. Except it was FALSE, measured:

```text
collision_aabb / SimpleActorGeometry — production call sites
  home body footprint publish        avatar/body_integration.rs
  actor body footprint publish       features/ecs/actors/update.rs
  the debug overlay                  game/ambition_app/src/dev/…/gizmos.rs
  boss callers                       ZERO
```

⇒ ⭐⭐ **a stated boundary is only worth what its accuracy is worth.** This one
was load-bearing in the wrong direction: it would have sent a later reader to
duplicate a helper rather than share it. The header now says what the module
actually holds, and records the measurement so nobody re-derives it.

▢ **THE CARVE IT IMPLIES IS REFUSED FOR NOW, WITH CAUSE AND A SIZE.** The
universal half of `attack_geometry` wants to live below the boss crate beside the
other body vocabulary — but `CombatGeometry` names `ActorSpriteMetrics` and
`AnimationSelection`, both boss-crate types, and the edge runs
`boss_encounter → characters`, so `ambition_characters` cannot reach any of it.
⇒ moving the trait means moving three things, not one: a D33-shaped slice, not a
file move. ⭐ **unifying the publish first makes that carve strictly smaller** —
one call site to move instead of two.

▢ **AND ONE THING THIS FOUND ON THE WAY, MEASURED RATHER THAN ASSERTED: a
POSSESSED FLYER CANNOT REACH ITS OWN TOP SPEED.**

A possessed body does **not** change roads — possession is brain transfer, so the
body keeps `Without<PlayerEntity>` and stays on the ACTOR road with
`Brain::Player` driving it. That road's flight limb OVERWRITES the input axes with
the brain's `velocity_target` projected onto the frame and normalised by
`flight_speed`:

```text
brain/player.rs:120   velocity_target = stick_local → world × max_run_speed
integration.rs:~350   axes = (velocity_target → local) ÷ flight_speed
                      flight_speed = max(chase_speed, max_run_speed, 1.0)
⇒ a fully deflected stick reaches max_run_speed / flight_speed of the available
  deflection — full only while chase_speed ≤ max_run_speed
```

⭐ **so steering WORKS** (the round trip is local → world → local, which is why
nobody has reported it), and only the MAGNITUDE is wrong: a human possessing a
body whose `chase_speed` exceeds its `max_run_speed` flies it at a fraction of
what the same body does under AI. ⚠ **latent on the shipped cast** — only two
catalog rows author `chase_speed` at all, and no flyer among them — so this is a
model defect rather than a live one, and it should be fixed when the flight limb
is next touched rather than chased now. ⭐ it is exactly the milestone's own
sentence made concrete: *"the protagonist should be special because of current
control assignment … not because generic simulation has a hidden coordinate
system."* Here the hidden coordinate system belongs to the AI.

⛔ **do not manufacture another helper extraction to make the function shorter.**
"Bevy accepts the signature" was never the goal, and neither is a line count. Take
a phase extraction only when it reduces mixed authority.

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
local view). ✔ **and the SELECTION shipped 2026-08-17** — Gameplay → *Camera
Frame*, world-fixed / player-relative, written onto the view component from
`GameplaySettings::camera_reference_frame`. ⭐ the input pairing needed no second
setting: a player-relative view collapses every `InputFrameMode` onto
body-relative as an identity, so the movement/aim rows report inactive instead of
being clobbered. ⛔ **do not continue it as a standalone campaign.** C5 — camera policy
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

- ✔ **D126 — CLOSED 2026-08-14. Three capabilities were DECLARED and consumed by
  nothing; the honest answer was a deletion or a report, never a wire.**
  `resolve_axis_repair` separates feasible contacts from infeasible ones
  (`AxisConstraintConflict` on `FrameEvents`, deliberately unread — damage, death
  and respawn are Ambition policy); `step_kinematic` and
  `ActorControlFrame::drop_through` deleted for zero production callers.
  ⛔ the REJECTED fix is the part worth keeping: *"sort by penetration depth"*
  turns the red test green while concealing the physics. Item 4 moved to D115 — a
  one-way moving platform is **not a `bool` away**, because
  `one_way_landing_from_previous_feet` compares a PREVIOUS feet coordinate against
  a CURRENT face, so a rising elevator would steal a landing off a stale line.
  ⛔⛔ tooling footgun: `scripts/rollback_codec_shape.py` skips any path containing
  `/.claude/`, so from a worktree it sees ZERO codec files and `--record` blanks
  the baseline — record baselines from the MAIN tree only.
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
⭐ the CPU's `smash_dash_to_close` (⚠ GONE — the dash-to-close family was deleted at `a7b5ab681` when the smash CPU's hard approach became a SPRINT, D146-1) was already locomotion (full throttle) with
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
* ✔ **D143** — CLOSED 2026-08-18. The stage's `unarmed_melee` reaches a kit-less
  seat: the publisher was reading its own deferred `insert_resource` write, so
  the floor was `None` on the frame that decides the match. Unreachable from the
  grid today (all fourteen author tables); it was real for the next character
  seated without one, and the guard now fails if the floor goes missing.
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

- ▢ **D165 — THE CHARACTER AUTHORING PACKAGE IS PROMOTED, AND ITS FIRST SLICE IS
  A CANONICAL HEIGHT IN SHARED WORLD UNITS. (opened 2026-08-17 by maintainer
  direction)**

Plan: [`engine/character-authoring-package.md`](engine/character-authoring-package.md)
— 1,061 lines, nine settled-direction sections, twelve named open questions, and
until today referenced by no ledger **by its own instruction**. Jon promoted it:
*"a character pack might be a good way to author character height in some shared
world units so we can get a sense of the scale at which characters should
render."*

⭐⭐ **THE SLICE IS CHOSEN SO THE PACKAGE FORMAT IS DEFINED BY A REAL CUSTOMER**
rather than argued up front — which is exactly what those twelve open questions
were waiting for. The customer is three of Jon's own reports that are one defect.

```text
today   collision_scale multiplies each sheet's OWN frame size
        heavies 1.95 · other pirates 1.60 · robot 2.10
        ⇒ the LARGEST number is the character who reads chibi, and the three
          numbers cannot be compared with each other at all
wanted  a character DECLARES its height in one shared unit; render size derives
```

**The four rulings that specify it** (all 2026-08-17, in
[`maintainer-decisions.md`](maintainer-decisions.md)):

1. **the unit is ONE BASE-GRID PIXEL**, 16 to a tile — `defaultGridSize: 16` is
   confirmed across the shipped worlds, and it is what collision AABBs already
   effectively use, so this is mostly **declaring what is already implied**.
   ⚠ a quality tier scales the ART, never the declared height.
2. **height is a CONTRACT**: art scales to it, so the cast is consistent by
   construction and a badly-framed sheet cannot make a character huge. A tight
   tolerance **WARNS** when the scale factor drifts. ⛔⛔ **warns, does not
   refuse** — that word is Jon's, and it is what separates this from a gate.
   ⚠ *"maybe a tight tolerance"* attaches to the NUMBER: pick it from the measured
   population and state it; ⛔ do not invent a round one.
3. **landmarks are OPTIONAL SLOTS** — head/feet/hands/sockets authored where
   useful, and every consumer must work without them. ⛔ never make one required
   to satisfy a consumer; that inverts the rule. ⚠ *"we may eventually have
   skeletons available in game"*, and a skeleton subsumes hand-authored landmarks.
4. **promotion did not schedule the other eight milestones.** A slice becomes
   work when something asks for it.

⛔⛔ **BOSSES AND GIANT BODIES ARE IN SCOPE — ruled 2026-08-17, and it roughly
DOUBLES this slice.** A boss is a character that happens to be large: same units,
same contract, and a multi-part body declares the height of the whole SILHOUETTE.
The boss sheet path computes its own render height today
(`collision.max_axis * collision_scale`, authored at 4.5 / 1.8 / 1.6 / 1.25) and
must derive from the declared height instead. ⭐ taken deliberately over an
ordinary-cast-first slice, because an exemption meant to be temporary is exactly
the kind that becomes permanent — **an exemption list is a TODO list.**

⛔ **`collision_scale` stops being a SIZE knob and is NOT deleted in this slice.**
Its own doc says what it actually is — *"a multiplier on the actor's collision
AABB… authored per-character to compensate for the fraction of each frame the
character art occupies after auto-crop"* — i.e. a PADDING compensation being used
as a size control. Height replaces the second job, not the first.

⚠ **the known trap, measured on the earlier attempt**: sizing the quad from the
body bbox WITHOUT also cropping the drawn region was tried and reverted because it
stretches the art badly. And the observation file records that it needs **four**
coupled sites, not the three the design doc names — **there are two render-size
publishers, and fixing one leaves both of the characters Jon complained about
untouched.** ⇒ find both before editing either.

⇒ **acceptance is Jon's three reports, not a number**: the snake and AI slop, Sanic
in his own game, and the cove pirates against the robot. If declaring heights does
not settle them, the quad-from-bbox route comes back **with evidence** rather than
with an argument.

✔ **SLICE 1 LANDED 2026-08-18 — the vocabulary exists and its first customer
states its height.** `Vitals::canonical_height` (world pixels, 16 to a tile) plus
`world_per_pixel_for_height`, and the robot lineage now DECLARES 48 rather than
spelling the division. ⭐ **no behaviour changed**: 48 / body_px.y is exactly what
it computed before, which is the point of a slice that introduces a unit.

✔ **MARY-O'S ART NOW SCALES FROM A RIG, LANDED 2026-08-18** (renderer submodule
`2813531`) — the half that had to happen in the sheets before one declared height
can drive them. Her parts hung off the GROWN form's absolute offsets, so
re-proportioning the short form to one brick broke them one at a time: seven
"X doesn't follow the body" defects, each visible only in a render. `FormRig`
states where parts belong as fractions of the form's own authored size, solved
from the approved grown form — which stays **byte-identical** through the change,
verified frame by frame against a control render. The front/death pose was the
last drawing still on absolute offsets, which is why it was the one Jon reported
unfixed.

⛔⛔ **the correctness argument is WHICH INPUT OWNS WHAT, and two wrong splits
both rendered fine at a glance.** The pose owns placement (it already carries
crouch, lean and bob); the form owns the proportions the fractions multiply.
Deriving the hip from the form alone silently dropped crouch — 14,780 pixels
moved on the approved form — and scaling x by the crouch-widened width moved
every skid and crouch frame. Neither was visible without differencing renders.

✔✔ **AND MARY-O IS ONE BRICK NOW — the rescale that was "blocked" was a UNIT
CONVERSION.** `SMALL_FORM_HEIGHT` is `T` (32 world units, one tile) instead of
48, so she stands one block small and two grown, which is Jon's ruling.

⛔⛔ **THE BLOCKER WAS AN ARITHMETIC ERROR IN THIS LEDGER, AND IT COST A REVERT.**
The earlier attempt read Jon's *"16 units"* as this demo's world unit, set the
constant to 16, watched the flagship vault break, and concluded a level-wide
rescale was owed. But `defaultGridSize: 16` is the LDtk AUTHORING grid; the
generated 1-1 those vault measurements live in is authored on `T = 32` world
units per tile. So one block here is 32, and 48 was never "three blocks" — it
was 1.5 tiles.

```text
read as 16   small 16, grown 32   vault clearance 76 vs 32 -> mouth floats 44 above her   ✗
read as T    small 32, grown 64   vault clearance 76 vs 64 -> fits, 12 inside a 16 slack  ✔
```

⇒ ⭐⭐ **the level needed no re-authoring at all** — `a_pipe_you_enter_always_has_a_pipe_you_come_out_of`
passes at the new size, and so does the whole workspace. The *"60 units of reach"*
that this row called a second blocker was measured in the same mistaken unit.
⭐ **what DID have to land first was the art**, and it did: at 1.40:1 no single
scale reaches 1:2 without widening her 1.43x, which `her_forms_are_all_the_same_width`
refuses on a gameplay rule. The rig work made the ratio exactly 2.0.

⚠ **two things this forced are WAITING ON JON**, recorded as §14 in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md): the shared
collision width went 64 → 56 px (one width for every form and "narrower than the
drawing" are together decided by the narrowest form), and her one-brick box has
6 px of headroom above her hat because the box top comes from the height contract
rather than the art.

⭐⭐ **AND THE UNIT WAS ALREADY THERE, WHICH MAKES THE RULING CHEAPER THAN IT
LOOKED.** `DEFAULT_PLAYER_BODY_HEIGHT` is 48 world pixels — exactly three tiles at
`defaultGridSize: 16` — and the field's own doc calls them *"world pixels"*. So
Jon's *"one world unit = one base-grid pixel"* DECLARES what the engine already
used; nothing converts.

⛔⛔ **WHAT WAS MISSING WAS NOT A UNIT BUT AN AUTHORED NUMBER — three characters
were each deriving the same scale by hand, and not even on one axis:**

```text
player_robot_lineage.rs:203   world_per_pixel = DEFAULT_PLAYER_BODY_HEIGHT / px.y   ← height
ai_slop.rs:177                world_per_pixel = AI_SLOP_BODY_WIDTH      / px.x      ← WIDTH
snake.rs:412                  snake_world_per_pixel(), an opaque helper
```

⇒ the slop's HEIGHT is whatever its art's aspect ratio produces, because nobody
ever stated it. That is the same defect as `collision_scale` one layer up.

⛔⛔ **AND THE NEXT SLICE IS NOT THE ONE THIS ROW ASSUMED — Jon's own note
redirects it.** *"Their collision bodies are the right size in world units now
(snake 1.00x Mary-O's width, slop 1.09x), and what is left is that the drawn quad
is 2.46x the body inside it."* ⇒ **the COLLISION derivation is already correct for
the characters he complained about; the QUAD is what is wrong**, and the quad
comes from the legacy road (`render_size`, sprite metadata carrying
`collision_scale`) rather than from the art. ⭐ so slice 2 is **moving those
characters onto the published road** — `BodySource::SpriteAuthored`, where the
collision box, the quad and its offset all follow from the art at one scale and
none can drift from the other two — not authoring more heights.
⚠ read `spawn_actors.rs:733`'s comment before touching it: the shared
`ActorRenderSize` exists precisely so a hostile flip cannot re-apply
`collision_scale` and balloon the sprite a second time.

⛔⛔ **AND THAT NEXT SLICE IS WRONG — MEASURED AND PHOTOGRAPHED 2026-08-18, and
the disproof is cheap enough that it should be read before anybody starts.**
*"Moving those characters onto the published road"* cannot close the 2.46x,
because **the snake is ALREADY on it and still measures 2.46x.**

```text
tag_mary_o_snakes   inserts SpritePosedBody          -> 2.46x
tag_mary_o_ai_slop  did NOT; I added it and shot the room:
                    drawn slop 48x55 px -> 48x54 px  -> UNCHANGED
```

⭐⭐ **the number is a FRAME-vs-BODY fact, and it was already measured, named and
ratcheted** by `enemy_quad_matches_its_box` (`QUAD_OVERHANG_LIMIT = 2.47`) —
which is where Jon's *"2.46x"* comes from. Its own doc states the mechanism:
the snake's sheet publishes a body of **117 x 52 px inside a 128 x 128 frame**,
and `PosedBodyGeometry::render` is *"the whole sheet frame"*. ⇒ **the quad is
SQUARE while the animal is 2.25:1**, at every scale, on either road. ⛔ so no
amount of re-wiring who publishes the size changes it; what changes it is
drawing the body's SUB-RECT, which is the art-crop the row already records as
tried-and-reverted for stretching the art.

⇒ **the honest next step is the trim**, not a road change: the sheets already
publish per-frame rects with `off`, so the quad can be the rect and the offset
can place it — which is also why the earlier attempt stretched, having done the
first half without the second.

✔✔ **THAT OPEN THREAD IS EXPLAINED — 2026-08-18, and the answer is that the
slop's SIZING WRITES A MIRROR, NOT THE AUTHORITY.** Two single-variable poisons
of the same body had disagreed:

```text
AI_SLOP_BODY_WIDTH 28 -> 60      drawn slop 48x55 -> 48x55 px   ZERO change
body.half_size     -> 4.0        drawn slop 48x54 -> 18x21 px   follows it
```

⭐ **measured on the live entity rather than on the sizing function**, which is
what every existing guard here asks and why none of them could see it:

```text
ai_slop_half_size()        28.0 x 18.2     what the constant derives
CenteredAabb @ tick 2      28.0 x 18.2     tag_mary_o_ai_slop's write LANDS
kin.size / BodyBaseSize    73.87 x 48.00   the AUTHORITY — never written
CenteredAabb @ tick 400    73.87 x 48.00   re-derived from the authority
```

⇒ `tag_mary_o_ai_slop` does `body.half_size = ai_slop_half_size()` on
`CenteredAabb`, which is a DERIVED MIRROR; `reset.rs` does
`aabb.half_size = em.kin.size * 0.5`, so the spawn size comes back. The constant
reaches the mirror for two ticks and never reaches the body. **Both readings were
true**: the drawing does follow the box, and the constant does not reach it.

⛔⛔ **and the guard beside it is structurally blind to this.**
`the_ai_slops_box_has_the_shape_its_sheet_publishes` asserts against
`ai_slop_half_size()` — the FUNCTION — and its own doc says it was written that
way on purpose after a first draft that recomputed from the sheet. Asking the
function proves the arithmetic; only asking a spawned slop proves the body. ⇒ the
same lesson the human-grab defect taught one layer up: *a test that starts
downstream of the wiring cannot see the wiring.*

▢ **the fix is one line and it is NOT TAKEN HERE, on this row's own rule.**
Writing the authority (`kin.size` + `BodyBaseSize`) instead of the mirror makes
every slop 28 x 18.2 rather than 73.9 x 48 — a **2.64x shrink** in a level Jon
plays. This row already says *"how big a slop should be is a taste call for
whoever is looking at the running game"*, so the size is his and the defect is
that the intended value never took effect. ⚠ **`SpritePosedBody` is NOT the
overwriter** — checked and it is absent on all twelve slops, so the sprite road
is not involved and the 73.87 x 48.00 comes from the spawn.

⛔⛔ **AND THE ONE-BRICK RESCALE HALVED THE SNAKE, WITH NOTHING SAYING SO.**
`snake_body_width()` derives from `mary_o_body_width()`, so when she became one
brick the snake followed her down: `world_per_pixel` 0.35 -> **0.182**,
collision 41 x 18 -> **21.3 x 9.5**, which is **0.30 tiles tall**. ⭐ **the
ratchet beside it could not notice**: it pins the quad/body RATIO, and a ratio
is scale-invariant, so it read 2.46x before and after. ⇒ *a value derived from
another character moves when that character does, and a ratio test is
structurally blind to it.* The two docs that quoted the old sizes are corrected;
whether a third-of-a-tile snake still reads as an enemy is a look-at-it call, and
⛔ the constant to change, if any, is HERS, not the snake's.

- ✔ **D164 — CLOSED 2026-08-18. Two top-level plans looked stranded and were
  already indexed; the audit had enumerated the wrong three files.**

[`sprite-residency-and-live-quality.md`](sprite-residency-and-live-quality.md)
(steps 2–5 open) and
[`frontend-audio-is-per-experience.md`](frontend-audio-is-per-experience.md)
(one open step) are both listed with ▢ entries in
[`tracks.md`](tracks.md) — **since `594a548bf`, 2026-08-13, four days before this
row was opened to say they were reachable from nowhere.**

⛔⛔ **the row's evidence was "referenced by neither `queue.md` nor `roadmap.md`
nor the README", which is TRUE and does not support the conclusion.** `tracks.md`
is the standing backlog, and the README's own orientation points at it for
exactly this class of work — so the one index that should have been checked first
was the one not in the list. ⇒ **an ABSENCE claim is only as strong as the set of
places you looked, and a hand-written set of places is a guess.** Same shape as
the doubled zone-name count in D161: the method was wrong, not the arithmetic.

⭐ **what the row got right and is worth keeping**: whoever picks up Jon's
*"changing video quality swapped robot v3 for v2"* should read the residency plan
first, because step 1 is what made a quality change retire and re-materialize
bodies already on screen. And his *"when you challenge PCA in the C4 symmetry
room we should change the music to a smash track"* is the consumer the audio
plan's open step is waiting for — ⚠ two mechanisms exist at different layers
(an encounter's authored `music_track` vs route declarations), the PCA challenge
is an encounter, and that doc is the one saying a process-global switch is the
wrong shape.

- ✔ **D163 — CLOSED 2026-08-18. The validator's errors are 0 and its loudest
  warning no longer flags a designed relationship. (opened 2026-08-17)**

```text
                    was                              now
error:              30, ALL false positives          0
spawn_overlap       8, every one a rider on a mount  0 (mounts exempt; real overlaps still fire)
missing_level_wall  portal_lab false + genre pits    5, all genre pits (a bottomless pit IS the design)
editor.shape        8 entities unplaceable in 2      6, all `SurfaceRamp`, deliberately deferred
```

⛔ **the one thing left is a PRODUCT CALL, not a defect**: `SurfaceRamp` has a
converter, a winding oracle and 0 placements in any world, so whether to invite
authors into an unused capability is Jon's. It is now the ONLY thing that
warning says, in every world, which is what makes it a signal.
⚠ the `sanic_sandbox` off-grid origin is AUTHORED — its spec declares
`world_y: 3000` and the live level agrees — so it is not drift; ▢ moving it means
editing spec and level together, for one level in the whole project.
▢ **and one question came out of this**: who owns a level's POSITION, the area
spec or `world auto-layout` (§16 in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md)).

⭐⭐ **THE LESSON THIS ROW EXISTED FOR, KEPT:** a validator whose errors are 100%
noise teaches you to read a red exit as noise — and this one talked a session
into reaching for `entity delete` on the shark-riding pirates. ⇒ **every finding
below was a proxy question standing in for the real one**, which is the pattern
rather than four coincidences.

⛔⛔ **I FILED THIS ROW WITH A HEADLINE THAT WAS WRONG, AND THE WRONG VERSION WAS
ACTIONABLE — that is the finding worth keeping.** I reported *"two pirate-sky
rooms ship seven DUPLICATED enemy spawns"* and was about to delete them through
`ambition-ldtk entity delete`. They are not duplicates:

```text
pirate_sky_lookout  [192,240]  Pirate Raider   mounted_on → the shark below it
                    [192,240]  Burning Flying Shark
                    …4 such pairs (3 Raiders + Iron Mary)
pirate_sky_arena    …3 such pairs, which is why "every spawn" looked doubled
```

⇒ **a rider and its mount are AUTHORED at the same pixel**, and the raider's
`mounted_on` field names the shark's entity iid. Deleting either half would have
destroyed the shark-riding pirates — **the exact content Jon reported missing
once already** (*"The pirates in the pirate sky no longer ride their sharks"*),
restored from git and guarded after a 2026-07-06 editor session dropped the
mount refs.

⭐ **only the field comparison stopped it.** Position-identical was the whole of
my evidence, and two entities at one point is what a mount IS. ⇒ **compare
FIELDS before calling two entities duplicates**, and treat any "clean up this
redundancy" impulse in authored content as needing a reason the AUTHOR would
recognise.

## What the validator actually reports

```text
30 error:  lines   ALL false positives — 4 cross-world LoadingZone targets that
                   a single-file validator cannot resolve, and 26 that exist
                   only if you validate the raw map_assets copy instead of the
                   canonical symlinked path (the entity manifest sits beside
                   the symlink)
spawn_overlap      FALSE POSITIVE for mounts — it does not know a rider sits on
                   its mount, and it fires on every one
missing_level_wall GENRE-DEPENDENT — fires on mary_o_1_1 and sanic_speedway
                   where a bottomless pit is the design
```

▢ **the two that survived scrutiny — and the first one did not survive a second
look:**
1. ✔ **`portal_lab`'s bottom edge was a FALSE POSITIVE, and the headless probe
   had already said so.** The room is walled on three sides and reported open at
   the bottom because `missing_level_wall` probes only the OUTERMOST cell row —
   while portal_lab's **full-width floor sits five rows above the boundary**,
   with unreachable empty margin below it. That is why the body rested at
   `(92, 872)` and never fell: ⭐ *"it does not fall from spawn"* was the
   answer, not a caveat.

   ✔ fixed by asking the real question for that side: a floor blocks a fall
   **wherever it is**. ⚠ **only the BOTTOM gets this** — the same idea on
   left/right (a full-height solid column) is much NOISIER, measured at **46
   open sides instead of 6**, because a corridor's side wall legitimately has a
   doorway gap. A floor is continuous by nature; a wall is not.
   ⭐ both terms observed: `portal_lab` stops warning, `mary_o_1_1` and
   `mary_o_1_3` still do — and those are the genre-dependent bottomless pits
   this row already excused.
2. **`SurfaceRamp` has no editor definition**, so a supported engine entity
   cannot be PLACED by an author. ⭐ **verified both ways**: the converter is real
   (`conversion/entity_converters.rs`, 5 sites, plus a winding oracle), and
   `sandbox.ldtk` carries **33 entity defs with `SurfaceRamp` not among them**.

   ⭐ **the spec is already written, so this is a lookup not a derivation** — the
   converter documents exactly what a def owes:

```text
radius       px, REQUIRED, must be > 0 (the converter errors otherwise)
orientation  one of four RampOrientation names, default FloorToRightWall
segments     polygon resolution, default 8, minimum 2
```

   ⚠ **left undone deliberately.** Writing defs means touching every world's
   `defs.entities`, and this project already pays for LDtk edits that go through
   the wrong road; it is also a product call whether to invite authors into a
   capability that has gone unused since it was written. Fix with the tool's own
   `ambition-ldtk def register-entity`, not by hand.

   ⛔⛔ **AND THAT REASONING COVERS ONLY `SurfaceRamp` — MEASURED 2026-08-18, the
   warning names EIGHT entities and the other seven are a different problem
   entirely.** They are defined AND placed in four of the six worlds:

```text
entity           placed   defined in
GravityZone          10   hall_of_characters · sandbox · sanic_speedway · mary_o
GroundItem           16   ″
Portal               14   ″
PortalGunSpawn        3   ″
ShrineSpawn           1   ″
SurfaceChain          4   ″
SurfaceLoop           1   ″
SurfaceRamp           0   — nowhere
```

```text
world                          defs   missing
hall_of_characters / sandbox     33   SurfaceRamp only
mary_o                           35   SurfaceRamp only
sanic_speedway                   33   SurfaceRamp only
intro.ldtk                       26   ALL EIGHT   ← the flagship
you_have_to_cut_the_rope.ldtk    26   ALL EIGHT
```

⇒ **an author working in the FLAGSHIP world cannot place a Portal, a
`GravityZone` or a `GroundItem`** — supported, converted, and used next door in
`sandbox`. That is not a product call about an unused capability; it is two
files out of step with four.

✔ **RECONCILED 2026-08-18.** The seven were copied into `intro` and
`you_have_to_cut_the_rope` through `def upsert-entity`, from a spec extracted
from what `sandbox` already declares — including each entity's authoring `docs`,
which is the part an author actually reads. All six worlds now carry 33+ defs
and differ only in `SurfaceRamp`. Verified: EntityRefs unchanged (4 and 0),
entity instances unchanged, every world validates with **0 errors**.
⚠ **`SurfaceRamp` stays out on purpose** — 0 placements in any world, so the
row's original *"a product call whether to invite authors into a capability that
has gone unused"* applies to it and only it. That is now the ONE thing the
`defs.entities` warning reports, everywhere, which makes the warning a signal
again.

⇒ **the row is the instrument, not the content.** A validator whose errors are
100% noise and whose loudest warning flags a designed relationship is worse than
none: it taught me, in one sitting, to reach for `entity delete` on a feature.

✔ **`spawn_overlap` KNOWS ABOUT MOUNTS, 2026-08-18.** A pair joined by a riding
reference (`mounted_on`) is exempt, because position-identical is what a mount
IS and only the FIELDS can tell a relationship from a duplicate. MEASURED both
terms on shipped content: sandbox **5 → 0**, intro **3 → 0**, mary_o 0 → 0 — so
every `spawn_overlap` warning the project was carrying was a rider on its mount.
⭐ the paired test is the one that matters: two unrelated spawns at ONE pixel
still warn, so the exemption keys on the reference rather than on the coincidence
it exists to catch.

⛔⛔ **AND A SECOND INSTRUMENT IN THE SAME TOOL WAS READING NOTHING AT ALL —
found 2026-08-18 while chasing the off-grid `sanic_sandbox` origin.**
`level diff-specs` is the CI-friendly check that an area spec's
`world_x`/`world_y`/`px_wid`/`px_hei` still match the live LDtk. It loaded YAML
only:

```text
--all globs *.yaml         22 files, and it SKIPS every one as "not an area spec"
world_x in a .yaml spec     0 of 22
world_x in a .ron  spec    53 of 59      ← every area spec is RON
a .ron passed explicitly    crashes in the YAML scanner on the RON comment
```

⇒ **it reported success by finding nothing to check.** ✔ fixed: the loader reads
RON through the tool's own `ron_parse`, and `--all` globs `.ron` and `.json` too.

⚠ **and turning it on reveals real drift, so it is NOT wired into CI yet**: 52
specs differ, 2 match, **78 coordinate mismatches** — some enormous
(`volatile_cache` spec says `world_x = 72000`, live is `2048`). ⚠ 13 of the 52
are specs describing levels in ANOTHER world file, which this command cannot see
because it takes one `--ldtk`; those are a usage limit, not drift.

▢ **the question underneath it is who OWNS a level's position** — the spec, or
`world auto-layout` which arranges levels by their LoadingZone graph. The tool's
own message says *"live LDtk wins"*, which suggests the specs' coordinates
stopped being authoritative and nobody re-recorded them. ⛔ do not bulk-rewrite
52 specs to silence it before that is answered.

⭐ **the `sanic_sandbox` off-grid origin is NOT drift**, which is what sent me
here: its spec declares `world_y: 3000` and the live level agrees. The half-tile
offset is AUTHORED, in `specs/sanic_sandbox_area.ron`, and moving it means
editing spec and level together — recorded rather than done, since one level in
the entire project is off-grid and it is a sandbox test room.

✔ **AND THE 30 ERRORS ARE 0, 2026-08-18 — RESOLVED, NOT SUPPRESSED.** Both
causes were the validator being handed less than the runtime has:

```text
4 cross-world targets   the runtime MERGES sibling worlds; a validator given one
                        file cannot see them ⇒ secondary worlds now DEFAULT to the
                        siblings beside the file (`--no-sibling-worlds` opts out)
26 unknown entities     `mary_o.entities.json` sits beside the SYMLINK in the game's
                        assets dir, not beside the real file in `map_assets` ⇒ the
                        sidecar search now looks in both, so the verdict depends on
                        the world and not on which spelling of its path you typed
```

⛔⛔ **the default belongs in the LIBRARY, not a CLI parser — it was in the
parser first and `repair` walked straight past it**, so a `entity set-field`
write still failed on the errors it was meant to clear. Every entry point
(`validate`, `repair`, the write-side `repair_and_validate`) reaches one line.

⭐ **both terms measured, because a check that always passes is the failure mode
here**: `--no-sibling-worlds` still reports the cross-world targets, and hiding
the sidecar still produces all 26. ⚠ the world MANIFEST is the runtime's real
authority and is Rust, so siblings are a proxy — exact today (the manifest lists
precisely the four files in `assets/worlds`), and if they diverge a world on disk
but absent from the manifest would pass here and fail at load.

- ▢ **D162 — REOPENED 2026-08-18: the SheetRegistry dismissal rested on a
  reporter that never ran, and running it finds THREE real ones.** (was CLOSED
  2026-08-17, four standing boot warnings triaged)

⛔⛔ **"✔ DISMISSED — no character id collides" was measured from a silence that
meant "I did not run".** `report_shadowed_character_sheets` owns the
catalog-aware half — the crate's own comment says the sheet crate *"cannot make
this call and must not learn to"* — and it is a `Startup` system.
**`init_sheet_registry` is ALSO a `Startup` system, and Startup is UNORDERED**, so
it ran with `Res<SheetRegistry>` absent, took the `else { return; }` on its
`Option`, and printed nothing on every route. Instrumented to emit one line per
shadowed target it printed **ZERO on both `mary_o_gameplay` and
`ambition_gameplay`** while the registry logged 39 shadowed targets in the same
boot.

⇒ moved to `PostStartup`. ⚠ **and "the catalog knows this name" turned out not to
be the question either** — of the 39, `toon` (15) is not a catalog id but
`robot` (15), `goblin` (8) and `sandbag` (1) all ARE, because those names are
shared RIG adapters that happen to also name a character. That filter would have
reported 24 legitimate rig shares as defects.

⭐⭐ **THE HARMFUL CASE IS THAT THE CHARACTER'S OWN SHEET LOST** — which is
exactly the founding defect (`pirate_heavy_broadside_bess` loaded the right image
and cropped it with a stale manifest's grid, a day and a bisect). Asked that way,
against `ShadowedTarget::loser_image`, **three survive and every one is real:**

```text
robot     robot_spritesheet.png      256x256  LOSES to robot_archivist        230x256
goblin    goblin_spritesheet.png     239x253  LOSES to goblin_brute_hammer    232x256
sandbag   sandbag_spritesheet.png    128x128  LOSES to sandbag_armored_review 256x256
```

⛔⛔ **AND THE HARM IS NOT DEMONSTRATED — I nearly wrote that it was.** The
obvious next sentence is *"so v0 the robot crops with the archivist's grid"*, and
it is WRONG: `record_for_target`, which `posed_body_geometry` and the animation
path use, is backed by `record_index()`, and that index keys by **filename root**
— it even overwrites `record.target` with the file root. **The character geometry
road cannot collide.** What collides is the target-keyed `SheetRegistry`
RESOURCE, and its four consumers resolve slash, shrine, projectile and boss
sheets:

```text
bosses/sync.rs          registry.body_metrics(<boss target>)
slash_visuals.rs        registry.get(<slash sheet>)
shrine_visuals.rs       registry.get("shrine")
projectile_visuals.rs   sheets.get(<projectile target>)
```

⇒ **so: three real collisions, in a registry whose readers do not appear to
resolve those three names.** Checked: the boss road takes
`behavior.sprite_target.unwrap_or(&behavior.id)`, and no boss id or sprite target
is one of the five; the other three resolve `"shrine"`, a slash sheet and a
projectile target.

⭐⭐ **AND THE AMBIGUITY IS STRUCTURAL, NOT THREE STALE PAIRS — measured across
all 196 baked sheets:**

```text
196  baked sheets
 52  whose file_root differs from their target (authored against a RIG adapter)
  5  targets claimed by MORE THAN ONE file, between them 48 files:
       robot 18 · toon 16 · goblin 9 · sandbag 3 · ninja 2
```

⇒ **the target-keyed registry cannot answer "give me sheet X" for any of those 48
files** — which of them wins is load order. Its static twin `record_index()`
already keys by FILE ROOT (196 unique keys, no ambiguity), and
`from_baked_table_by_file_root` already exists.

▢ **so what is left is (a) retire the stale manifest in each of the three pairs
— a content call the registry cannot make — and (b) DECIDE THE KEYING**: for the
148 sheets where root == target the two are identical, every current consumer
looks up such a name, and switching would make this whole class impossible rather
than reportable. ⛔ **not taken unilaterally**: it changes what a shared engine
resource returns for 48 files, and the evidence above is what makes it a one-line
ruling instead of an investigation. Asked as **§19** in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), with both
options and their blast radius.
⚠ **the sandbag pair is the loudest if anything ever does resolve it**: a 128px
sheet cropped on a 256px grid.

⭐⭐ **two reusable halves, and the second is the one I nearly skipped**: *a
report that has never been seen to speak has not been shown to be silent* — and
*a real collision is not a real defect until you name the reader.*

✔ **AND A FIFTH BOOT WARNING, FOUND WHILE MEASURING THE ABOVE AND FIXED
2026-08-18.** `mary_o::quasar: overlay not attached: no current sprite frame
(atlas true, image loaded = false)`, twice on every Mary-O boot. ⭐ **the code
disagreed with the paragraph directly above it**, which reads: *"Both are 'not
yet' conditions that normally clear within a frame or two of the sprite loading,
so they are only worth a word if they PERSIST — which is exactly the case where
the overlay never appears and nothing says so."* Both arms `warn!`d on the FIRST
frame the condition held, so an ordinary texture decode printed the failure the
comment is about.
⇒ the condition is counted per candidate and reported ONCE past
`QUASAR_ATTACH_GRACE_FRAMES` (60 — a second at 60fps, far past any decode).
⚠ **poison-verified both ways**: unmodified, a 150-frame boot now prints ZERO
warnings and still logs `overlay attached`; with attachment made impossible it
prints exactly ONE — where the old code would have printed ~150.
⭐ same shape as the doorway diagnostic: **a comment that states the intent is
not enforcement of it**, and the fix is a threshold, not a filter.

⭐⭐ **THE CURRENT FLAGSHIP BOOT INVENTORY, measured 2026-08-18 on
`--route ambition_gameplay` after the above — SEVEN lines, and every one now has
a verdict:**

```text
sanic_sandbox off-grid Y     ▢ D163 — blocked on §16 (who owns a level's position)
GgrsSchedule redundant edge  ✔ WON'T-FIX — both memberships individually correct
SheetRegistry robot/goblin/  ✔ CORRECT and newly VISIBLE — the three above; the
  sandbag  (3 lines)            keying question is §19
room has 38 neighbours       ✔ LEAVE IT — `warn_once!`, names its constant, and
                                its author argued the case: *"a cap that quietly
                                drops work reads as everything is prefetched"*
npc_kernel_guide             ▢ NEW, and worth someone's attention
```

▢ **`NpcSpawn-0017` names `npc_kernel_guide`, which the composition has not
registered**, so it *"falls back to its catalog row's body with a borrowed kit"* —
and the message's own criterion says that is *"correct only for a BORROWED
character in a partial composition."* ⛔ **`ambition_gameplay` is not a partial
composition.** The character is authored into `hall_of_characters.ldtk` and
`sandbox.ldtk`, has its own spritesheet, and the intro road resolves it by id.
⇒ **a member of the room that exists to show the cast off is running on somebody
else's kit.** ⚠ whether it should get an authored `CharacterDefinition` — and
what kit — is a content call; ⚠ and check the D56 note first, which says the
Kernel Guide *"leaves it blank so kernel→goblin keeps its visual gag"*, in case
the borrow is the joke.

- ◻ **D162's original triage of the other three stands (2026-08-17).**

```text
SheetRegistry "39 targets"   ✔ DISMISSED — 166 targets, 4 geometry collisions,
                                every one a shared RIG; no character id collides
loading zone "did not fire"  ✔ FIXED (`ad82531b7`) — WARN only on a PRESSED door,
                                DEBUG otherwise. Verified 1 → 0 warnings, and the
                                door still transitions under `--press f`
GgrsSchedule redundant edge  ✔ WON'T-FIX — both memberships are individually
                                correct and Bevy drops the shorter edge
sanic_sandbox off-grid Y     → D163 — y=3000 is off by exactly half a tile, one axis
```

⛔⛔ **the first fix was wrong in a way worth keeping**: it filtered the message
away whenever no press was buffered, which would have silenced the instrument in
the very scenario that justified it — a broken binding is exactly the case where
the player pressed and `wants_interact` still reads false. ⇒ **a value a
diagnostic REPORTS is a bad value to gate it on.**
⭐ scale, so nobody opens a warning campaign: the whole engine holds 34 `warn!`
and 5 `warn_once!`, and boot prints four. This is per-call judgement about
whether the condition is ORDINARY, not a policy about warnings.

- ✔ **D161 — CLOSED 2026-08-18. No loading zone prints an authoring id any more:
  130 → 0, and a per-world ratchet in CI keeps it there. (opened 2026-08-17,
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
checks. ⚠ a lint is only worth adding if it can go green — **130 rows** (the
corrected count; ⛔ not the doubled 260) is a campaign, so it wants a ratchet (may
fall, must not rise) rather than a gate that is red on day one.

⛔ do not "prettify" the id by swapping underscores for spaces — that
manufactures prose the author never wrote, and `wake_to_raid` has no good
rendering. ⚠ and drawing nothing when the name looks like an id is a REGRESSION
for the doors that legitimately want a label; the answer is to author them.

✔✔ **AUTHORED 2026-08-18 — 130 → 0. Every loading zone in the project carries
prose a player can read.** All 151 that have a name have an authored one, and
each follows the convention its own level's author already used: `central_hub_main`
says *"to scroll lab"*, its basement says *"hazards"*, and the new names joined
whichever they landed beside. ⭐ the destination came from the zone's own
`target_room` — an authored fact — never from the zone id with its underscores
taken out, which `wake_to_raid` shows has no good rendering.

⛔⛔ **AND I WROTE OFF THE BULK OF THEM AS "a developer sandbox where a
diagnostic id is defensible" — WRONG, AND ONE QUERY SAID SO.** `sandbox.ldtk`
holds `central_hub_complex`, which the world manifest names as `entry_room`, so
17 were in the game's FIRST ROOM and 12 in the level below it, both already
carrying authored prose alongside — *"to scroll lab"* next to
`military_tower_door`, which is Jon's report exactly. ⇒ **a file's NAME is not
its audience.** The 44 that looked most peripheral were rooms the hub has a door
to.

✔ **the ratchet is `scripts/check_zone_name_ratchet.py`** (baseline
`dev/zone_name_ratchet_baseline.json`, now an empty map — the success state),
PER WORLD so one file cannot mask another, and it FAILS when it observes no
zones at all. ⛔⛔ it dedupes by real path: every world under
`game/*/assets/worlds/` is a symlink into `game/ambition_map_assets/`, which is
one naive `glob` away from the doubling that first published this population as
302/260/38. ⚠ and `None` ≠ `{}` in its baseline loader — an empty map is the
GOAL, and conflating it with "never recorded" would have made the check start
failing at the moment it was satisfied. Runs in CI.

⚠ **and one thing the row had wrong, found by looking:** the correct
`→ corridor` in that same frame is a **DebugLabel**, not a zone name. So the
opening room's real problem is partly DUPLICATION — the room already carries
authored signage — and naming the zone "to raid corridor" sits beside it rather
than replacing it. ▢ whether a non-Door zone should draw an unconditional world
label AT ALL (**24** named zones are `EdgeExit`, and those draw always) is asked as §17 in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), together
with what the measurement turned up beside it: **12 rooms carry BOTH authored
signage and always-on zone labels**, and `gate_stack_lower` has fourteen
DebugLabels. ⚠ `DebugLabel` is doing player-facing work — *"creator's basement
lab"* is one — so its name says one thing and its usage another. ⇒ polish, not
urgency: nothing on screen is an id.

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

✔✔ **REPLACED 2026-08-17 BY MAINTAINER RULING — and *replaced*, not deleted.**
Jon, verbatim: *"replace it, don't delete it. The hub benefits from an orientation
sign; only the authoring-language content is wrong."* His text, which keeps the
house `//` prefix on purpose:

```text
// CENTRAL HUB — ROUTES OUTWARD; BASEMENT ACCESS BELOW
```

⭐ landed on `DebugLabel-0019` in `central_hub_main` via
`ambition_ldtk_tools entity set-field` — **through the tooling, which he named as
part of the decision** (*"not by editing the JSON directly"*). Diff is 2 lines:
`__value` and its `realEditorValues` param, both halves of the one field.
⚠ **only `text` is player-facing**, so `name` stays an authoring id exactly as
`intro_wake_room`'s labels do; ⛔ and the EntityRef counts were checked before and
after (12 `entityIid`, 58 `mounted_on`, unchanged) because an LDtk write is the
operation that has silently nulled mount refs in this repo before.
⭐ **the deletion option is closed**: the hub wants signage, and *"only the
authoring-language content is wrong"* is the general test for the next one found.

- ✔ **D157 — CLOSED 2026-08-16. Mary-O had her whole smash moveset in her own
  platformer: `combat_actions` derived the attack slots from the MOVESET and the
  `ActionSet` and never read the `AbilitySet`, so her `abilities: Some([RunJump])`
  row bought nothing — twenty-three distinct swings reachable. Sanic was the
  identical construction and the identical break.** Landed: `combat_actions` takes
  the `AbilitySet` and ceilings the melee family with `abilities.attack`; the table
  stays attached and the GATE is what changed. ⚠ Projectile is deliberately NOT
  under it — a ranged verb is always an explicit authored grant, and folding it
  under `attack` would mean the only way to arm Mary-O's fireball was to arm her
  fists.
  ⛔⛔ **a test caught this and was argued away.** `…peaceful_kit…` asserted
  `moveset_len == 0`, went red at 17, and `3d3540546` rewrote it to AGREE with the
  17, reasoning that *"what keeps that off his own speedway is the ABILITY, not the
  table"* — a sentence that was an INTENT nothing implemented, written in four
  places and executed in none. Guarded by `mary_o_at_home_can_only_run_and_jump`,
  `the_run_button_throws_a_spark_only_while_she_wears_the_lantern` and
  `the_demo_body_cannot_trigger_a_single_move_from_its_own_smash_table`, all seen
  red on the pre-fix tree.

- ✔ **D156 — CLOSED 2026-08-16. The Patent Clerk faced backwards: his facing was
  authored THREE times (`CharacterSpec.facing` → the rig's `features.facing` → the
  SVG's `data-rig-facing`), all three said `west`, and nothing in Rust read any of
  them.** `gravity_aware_flip_x` was exactly `facing < 0.0` with no per-character
  term, so the engine assumed all ~800 baked sheets were drawn facing +x.
  ⛔⛔ it was a FORK, not a missing feature — `animate_bosses` has XORed precisely
  this term since the mockingbird. Landed `SheetRecord::authored_faces_left` and
  lifted `data-rig-facing` out of a Noether-specific validator onto `CharacterSpec`
  (`8c30de613`/`37ac258b6`, `fd4320071`; renderer `fac948b` → `9b445c5`).
  ⚠ near-miss: `SpritePackCatalog::to_sheet_record` synthesizes a record from ATLAS
  RECTS and cannot know which way pixels point, so each character would have been
  right from his own sheet and backwards again from the pack; the pack now inherits
  the base manifest's facing. ⚠ latent hazard: `facing: str = "west"` is the
  DEFAULT on `CharacterSpec`, so *"the rig says west"* can mean **nobody set it**.
  What protects the population is
  `every_baked_sheet_is_drawn_pointing_where_its_body_faces`, which pins the
  declaring set to exactly eight manifests.
  ▢ **still open, small**: the portrait tier declares no facing and was never
  checked (a question, not a known defect — both characters read correctly in
  game); and two PRE-EXISTING rig-validator failures stand at HEAD — Carl's
  canonical paint-slice order is out of order, and Noether's rig names
  `head_base`/`head_features` where `validate_one` requires `head`/`torso`. Only
  `validate` is red; the `build` path both regens use is fine.

- ✔ **D158 — CLOSED 2026-08-17, then SUBSUMED BY D159. Two taunting CPUs printed
  through each other because `stack_offset` was measured from each SPEAKER'S OWN
  HEAD and the speakers were at different heights** — floor-to-air is exactly one
  stack step, so every offset was distinct and every line was on top of another.
  A platform fighter has somebody airborne constantly, so that is the ordinary
  geometry, not a corner. ⚠ its whole bespoke stacking mechanism is DELETED: a
  bubble is now a `WorldLabel` in the one ranked placement pass (D159).
  ⛔ do not reintroduce a second system that places bubbles — two placement passes
  that cannot see each other is how this bug happened, and each could truthfully
  report *"no overlaps found"*.

- ✔ **D160 — CLOSED 2026-08-17 BY MAINTAINER RULING: the omission is deliberate.
  The cheap unit tier is a REQUIRED PRE-PUSH check, not a per-turn gate — two
  tiers on purpose.** Jon: *"keep the per-turn gate small … The workspace lib suite
  remains a required pre-push/finalization check … 'Gate' should continue to mean
  an executable gate."* Three tiers: `scripts/gate_suite.py` per turn, and it stays
  cheap · `cargo test --workspace --lib` pre-push, where *required* is not *gated*
  · feature-gated suites when you touch the subsystem.
  ⛔⛔ **DO NOT ADD `--workspace --lib` TO `gate_suite.py`.**
  ⛔⛔ **and this row was once closed on a false premise, which is why the ruling
  reads as it does.** It claimed the sweep was *"added to the stated gate"*; what
  landed was a PARAGRAPH IN `AGENTS.md`. ⇒ **two commands run in one turn is not
  one command invoking the other**, and *"in the gate"* means a line in
  `gate_suite.py` or a CI job and nothing else. ⭐ the rule that falls out: **when
  you add a check, name the TIER that runs it** — a check with no named tier is
  decoration. ⚠ the omission had hidden two suites red on `main` (repaired in
  `ea5ca88df`), both guards that were CORRECT when written and became wrong when a
  rule moved under them.

- ✔ **D159 — CLOSED 2026-08-17. A name plate printed through a taunt because a
  speech bubble was a FOURTH label family that never joined the one placement
  pass** — the nameplate pass and the bubble pass each truthfully reported *"no
  overlaps found"* about a frame in which *"George Booul"* sat inside *"Either you
  are on the stage or you are not."*
  ⭐ **the fix was to JOIN, not to invent**, and `label_layout.rs` had already
  written the diagnosis. `WorldLabelFamily` becomes `Signage · Fixture · Actor ·
  Speech`, ranked LAST on the module's own test — *which family can move without
  anything visibly jumping?* A plate is permanent furniture on a body the eye uses
  to track who is who; a bubble is born in motion and gone in 2.2s. D158's
  mechanism, its constants and `PendingSpeechBubble` go with it.
  ⭐ two defects fell out of the same shape: a 160-unit POINT RADIUS gated stacking
  for text that renders ~336 wide, and the pass's own displacement advanced in a
  fixed 11px quantum, so a budget of *"six steps"* bought two lines of clearance —
  both replaced by one honest `max_displacement_px`. Guarded by
  `a_name_plate_and_a_speech_bubble_do_not_print_through_each_other` (seen RED,
  both boxes at the same centre) and
  `the_bubble_yields_to_the_name_plate_and_not_the_other_way_round`, which pins the
  ranking ARGUMENT rather than describing it.

- ✔ **D155 — CLOSED 2026-08-16. Nobody got launched, and it was TWO bugs on the
  shared floor rather than the parameter tweak it looked like.** Probed live at
  1427%: magnitude, the write to the body and DI were all HEALTHY, and two things
  downstream of all three were wrong — each reproducing one half of the report.
  **(1) every authored launch direction in the game was VERTICALLY INVERTED.**
  `HitVolume::launch_dir` states `+y = gravity-down`, ~100 authored literals wrote
  against it, and `knockback_velocity` negated `y` anyway to satisfy a doc comment
  claiming the opposite — so every up-tilt, up-air and up-smash spiked its victim
  into the floor and every down-air lifted them.
  **(2) a launch big enough to TUMBLE was resolved as a LANDING on the tick it was
  applied.** The launched body kept its stale resting contact into the same step's
  `tick_knockdown`, which read `on_ground == true`, called that *touched down while
  still tumbling*, and zeroed the velocity: a 3269 px/s launch moved **zero
  pixels**. Fixed by clearing `ground.on_ground` when `launch_into_tumble` returns
  true — gated on the TUMBLE answer, so every body with `tumble_speed: 0.0` (all of
  Ambition) is byte-identical.
  ⚠ why it hid: every floor-game test set `on_ground = false` before launching, so
  the one situation a fighter is actually in when hit — standing on the stage — was
  never stepped. ⭐ the scaling guard measures RISE, not displacement, because the
  pre-fix build launched downward where the growth term still produced a big number
  the floor absorbed. Guards in `hit_response::launch_direction_tests`,
  `movement::tests::combat_actions` and `smash_in_the_host::launched`, each seen red
  pre-fix. ⭐ nothing smash-side was touched: both fixes are in
  `ambition_platformer2d_core`, so Ambition gets juggling from the same floor.

- ✔ **D147–D154 — ALL EIGHT REVIEW FINDINGS CLOSED 2026-08-17.** (external
  structural review, 2026-08-16, read against `2381e3a7e`.) ⭐ **6 of 6 reproduced
  when probed**; D151 leaves one named residual, recorded on its own row below.

⚠⚠ **PROVENANCE, and it is the durable half of this row.** The reviewer states
plainly that they *"couldn't independently run Cargo in this review environment"*
and *"treated the commits' reported green suites as evidence rather than rerunning
them."* ⇒ **every finding was a READING, not a measurement, and each was probed
before being fixed.** A finding that cannot be made to fail is a finding about the
reader, not the code — and two of these sharpened materially under the probe
(D151's bridge turned out to be load-bearing; D147's coupling too).

⭐ the reviewer's own ordering argument, worth keeping: the first three *"aren't
generic cleanup for its own sake: each one removes a piece of backend ceremony or
hidden state that future agents would otherwise repeatedly have to understand."*

⛔ **explicitly NOT findings** — the reviewer looked and cleared these: the volume
of new moveset lines (*"genuine fighter design"*), `levelled(SMASH_FIGHTER_KIT)`
as policy (Jon's own ruling), the camera work, and the smash submodule gitlinks.

- ✔ **D147 — CLOSED 2026-08-17 (`797aa480d`). Generic match activation knew the
  stocks ruleset's private latch** — D140 had fixed a never-ending second match by
  inserting `StocksMatchSettled(false)` inside the GENERIC activation road, which
  installed the resource even where that ruleset was never composed.
  ⭐ PROBED FIRST: the coupling was LOAD-BEARING (comment the line out and match
  two ends with zero winners), so this REPLACED it rather than deleting it. The
  latch is not a timeless global — it is *the stocks outcome for match X*, so it
  now carries the `MatchInstance` it is about and a new match reads as undecided BY
  CONSTRUCTION: nobody retracts it, nothing is ordered against activation, and
  activation names no ruleset. Guarded by
  `the_previous_matchs_verdict_does_not_settle_this_one`,
  `a_verdict_from_another_session_does_not_settle_this_match` and
  `adopting_a_seat_topology_does_not_un_decide_the_match` — the last is why the key
  is the activation's two facts and not the whole receipt.

- ✔ **D148 — CLOSED 2026-08-17 (`f0da10217`). A team victory announced the last
  surviving teammate instead of the team**: the card's own rule says a team keeps
  its name and only a side of ONE is swapped for the fighter, and it decided which
  by COUNTING THE BODIES standing — but an eliminated fighter is despawned, so a
  two-person team that lost a member early has one body at victory.
  ⭐ body residency recovering match-participant identity, the error this campaign
  keeps hitting: how many fighters a side HAS was frozen when the match was
  prepared (`PreparedMatch::seats_on_side`); how many are STANDING is the match.
  Guarded by `a_team_victory_names_the_team_and_not_its_last_survivor`, whose
  non-vacuity half asserts seat 1 left play BEFORE the decision.
  ⚠ the guard was rewritten once, and why is worth keeping: the first version let
  four CPUs fight between scripted launches, and a hitlag change in another crate
  flipped the winner. **A claim about the WORDING of a card must not depend on
  combat tuning** — every elimination is now caused by the test on a fixed schedule.

- ✔ **D149 — CLOSED 2026-08-17. Move VFX bypassed `FxRequest`, so fourteen
  movesets hand-paired every sound.** `dispatch_move_events` wrote
  `VfxMessage::Effect` directly, going around the one abstraction whose whole job
  is *"write ONE request for an effect's visual and its paired sound"* — so several
  characters carried a test whose entire job was checking that a content author had
  remembered a backend detail.
  ⭐ **the corpus decided the vocabulary**: of 145 authored `sfx(…)` calls, **74
  merely restated the default pairing** (deleted), **21 were `.loop` overrides**
  (kept — ten shipped rows pack their sound ONLY as the loop and would have gone
  silent), and 50 were independent voices (untouched). ⇒ `default | override |
  silent`, with the override arm proved non-speculative by the corpus itself.
  ⚠ the 74 were VERIFIED, not assumed — each compared against
  `effect_cue(FxId::new(effect))`; zero differed. Guarded by
  `a_paired_burst_is_heard_exactly_once`, seen RED on the shipped tables, which
  runs the real `dispatch_move_events` + `process_fx_requests` so *"what a burst
  already addresses"* is answered by the engine and never by a transcribed table.
  ⛔⛔ **the half-landing in between was a live feel bug**: `1e2aa6337` switched the
  arm and left the 74 restatements standing, so for one session every burst played
  its sound TWICE — individually correct, jointly a doubled jab, **and 412 app
  tests stayed green throughout**.
  ⚠ measured residue, deliberately left: two moves throw the same effect at `±x` on
  one frame and are heard twice. That is a burst COUNT, not a restatement; the
  `silent` arm would answer it and is not built for want of a customer.

- ✔ **D150 — CLOSED 2026-08-18. The stamp landed 2026-08-17; an audit that day
  found the same defect surviving in the tick before the stamp existed, and the
  stamp now happens at the projectile's BIRTH. A projectile changed allegiance
  when its firer despawned.**

⭐⭐ **RE-OPENED AND RE-CLOSED THE SAME DAY, on the review's ask to audit every
attack authorization still recomputed from the resident firer.** Four reads:

```text
faction + team   STAMPED (D150)                                   ✔
grudge           live read off the owner        AUDITED, KEPT — see below
`victim == owner_entity`  self-hit guard        healed handle, fine
`attacker: owner_entity`  KO credit             open, and D148's neighbour
```

⛔⛔ **and the fifth thing, which was a live bug**: `indiscriminate` was
`allegiance.is_none()`, while the comment beside it said *"a bolt that never had
a living owner"*. Those are different sentences. `owner_combat` wants a
non-optional `&ActorFaction`, so a NAMED owner that is merely gone comes back
`Err` — and on the shot's FIRST step that also means no stamp is taken, so the
bolt re-asks and re-fails every tick for the rest of its life. **A named firer
who vanished before the stamp got promoted to environmental hazard, permanently.**
⇒ `indiscriminate` now requires that no owner was ever NAMED. Demonstrated, not
argued: with the old predicate the new guard fails with the orphaned shot having
hit its firer's own teammate.

⭐ **the grudge was KEPT as a live read, and the reasoning is now on the type.**
`dissolve_settled_grudges` already ends a feud on a HEALTH rule that has nothing
to do with residency, so reading it off a living owner is right. ⚠ it was only
defensible once the line above stopped inverting: while a missing owner meant
indiscriminate, *"the firer is gone, so there is no feud"* became *"hit everyone,
including the bodies the feud existed to spare"* — a narrowing turning into the
broadest permission there is.

✔✔ **AND THE WINDOW IS NOW CLOSED PROPERLY, same day.** The defensive fix above
made an orphaned shot INERT rather than a hazard, which is safe and is not the
same as knowing whose attack it is. `stamp_new_projectile_allegiance` takes the
side where the entity is BORN — the conclusion the presentation half reached in
as many words (*"attribution belongs where the entity is born"*).

⛔ **it could not go where the presentation stamp went.** That one lives in the
two materializers themselves; those are in `ambition_projectiles`, which depends
on neither `ambition_combat` nor `ambition_characters` and so cannot name
`ActorFaction` or `MatchTeam`. So this is a monolith-side system installed into
the combat chain instead.

⛔⛔ **TWICE, and the reasoning that says once is enough has a specific hole.**
The first draft put it only before `step_projectiles`, reasoning that a player
bolt materializes after that step and first ticks next frame, when the stamp has
already run. True about STEPPING, false about the WINDOW:

```text
CombatSet::Materialize   materialize_projectiles_for_next_tick   bolt exists
CombatSet::Settle        take_eliminated_fighters_out_of_play     firer despawns
        ← same tick, and Materialize is EARLIER than Settle
next tick                stamp → step                            nothing to read
```

⇒ **the window is bounded by the firer's DESPAWN, not by the bolt's first step.**
A fighter eliminated on the tick they fire loses the body after the bolt exists
and before a single pre-step placement ever sees it. Second placement added right
after the player materializer, still inside `Materialize`.

⭐ **both directions pinned.** `a_shot_stamped_at_birth_survives_its_firers_elimination`
models exactly that tick with `run_system_once` — a plain `app.update()` cannot,
because it would step the bolt while the firer still lives and the stepper's own
first-sight stamp would take the fact, hiding whether the new system did anything.
Neutering the stamper turns it red; the safety guard beside it stays as the
negative term.

  **The original row, unchanged:** Allegiance was reconstructed EVERY TICK by querying the firing
  `Entity`, and the code stated the consequence as intended behaviour — so in a
  match a fighter fires, loses their last stock, the body despawns, and next tick
  the shot in flight turns on its own team. ⭐ the same occurrence-vs-entity
  distinction D125 is working through.
  ⭐ **the shot's PRESENTATION half already had the answer** —
  `inherit_projectile_presentation_sources` says the bolt *"routinely outlives the
  body that fired it, so the source is STAMPED at spawn rather than looked up at
  impact."* Same stamp for allegiance: `ProjectileAllegiance { faction, team }`,
  frozen on the first tick of flight, registered rollback state because after a
  rewind past the firer's death there is nothing left to re-derive it from.
  ⚠ two deliberate NOTs: the grudge is not frozen (a feud is something the firer
  holds now), and the faction stamped is the authored one — this changes a LIFETIME,
  not a rule. The parry re-own rewrites the stamp beside the owner handle.
  ⚠ named residue: a firer who dies between materialization and the bolt's first
  step leaves it unstamped forever; closing that means stamping at birth, which
  means an observer or moving the vocabulary across a manifest-deny boundary —
  not worth it for a one-tick coincidence.
  ⛔ **a NEW rollback name reddens TWO baselines**: `scripts/baselines/
  rollback-schema-baseline.json` and `game/ambition_app/tests/
  rollback_schema_baseline.txt`, the second of which a per-crate run never sees.

- ✔ **D151 — CLOSED 2026-08-17 (`d21031fc4`). `MatchAbilities`' `None → permitted`
  bridge turned PERMISSION into a GRANT**, in exactly the use the docs proposed for
  fighter individuality: with `permitted = kit + wall jump` to let the one
  character who authored a wall jump keep it, every UNAUTHORED character took the
  `None` arm and got it too.
  ⛔⛔ **PROBED FIRST, and the bridge was LOAD-BEARING** — the one real `at_most`
  adopter is the versus duel, and **neither duellist authored any abilities**, so
  the bridge was handing them their entire kit. Deleting it naively would have left
  both with nothing. Safe order: dress the cast first (`VERSUS_FIGHTER_KIT`, with
  `at_most(…)` now REFERENCING that constant so the ceiling and the kit cannot
  drift apart), then `apply` reads `authored.unwrap_or(AbilitySet::NONE)`. Exactly
  one test failed — the one that pinned the bridge — and it now pins the rule.
  ⭐ `levelled` is untouched (`granted == permitted`), so the smash stage's fourteen
  fighters are byte-identical. ⚠ the line's own doc had named the condition in
  advance: *"the day it is unreachable this line is `unwrap_or(AbilitySet::NONE)`."*
  ▢ **still open**: a ceiling must NARROW the body's own kit rather than REPLACE
  it, which `apply` cannot do because it never receives that kit — **that signature
  is the real work**, not the `unwrap_or`. Then guard that no seated character
  relies on the bridge, so the next unauthored fighter fails loudly.
  ⚠ `MatchBody::over` is the shape to copy: it takes the base the fighter brought
  as a PARAMETER, which is why the body authority never had this defect.

- ✔ **D152 — CLOSED 2026-08-17. Empowerment expiry was a per-game scheduling
  footgun**: a game had to install `run_empowerments` in its own schedule or a
  two-second invulnerability became PERMANENT — five adopters had each remembered,
  and smash's respawn protection is how it surfaced.
  ⭐ **the split that unblocked it**: contact-harm INTERPRETATION is a ruleset
  choice, but ticking and expiry are a domain INVARIANT. The engine installs the
  clock in a named `EmpowermentExpiry` set; the ORDER stays each game's, and
  `apply_contact_harm` stays optional and un-installed (Sanic still has none).
  ⚠ **one literal placement could not be preserved, and it is structural**: the
  five sat in THREE mutually exclusive phases, and one shared set has one position
  — per-game re-placement would be a schedule cycle. It sits at `GameplayEffects`,
  the LAST of the three, which makes it ordering-PRESERVING: every grant site is at
  or before it, and every consumer of what it writes reads from a phase that
  precedes all three. Nothing a body can observe moved.
  Guarded by `a_timed_empowerment_ends_in_a_composition_that_scheduled_nothing` —
  an app whose ONLY empowerment statement is the plugin, seen RED with the plugin's
  `add_systems` stubbed out.

- ✔ **D153 — CLOSED 2026-08-17. A missing required sprite page failed OPEN**: the
  permanent-spinner fix made an absent page `error!(…); continue;`, so a spec that
  required page 12 against a five-slot realization logged, omitted the page, let
  the barrier report Ready, and revealed the room with missing presentation.
  ⭐ **the failure travels IN the manifest, not out of the builder in a `Result`** —
  `RoomAssetManifest` gained `unresolved: Vec<String>`, and readiness counts each
  entry as SETTLED-and-FAILED so the reveal is refused and the source room stays
  authoritative. The shape argument, written at the field: the manifest already
  reaches both refusers AND is the prefetch cache's equality key, so a truncated
  realization can no longer be promoted as equal to a healthy one.
  ⭐ the arm widened by one honest case — a slot that EXISTS but holds a default
  handle is the same defect, and it is the reachable one. Guarded by
  `a_required_page_the_realization_lacks_refuses_the_room` (seen RED against a
  restored log-and-continue) beside the regression guard for the spinner fix it
  sits inside.

- ✔ **D154 — CLOSED 2026-08-17 (`97a5b76ea`). Authored VFX was only half
  body-local: the POSITION was transformed through committed facing and the body's
  gravity frame, and the ARTWORK was drawn world-upright regardless** — so a
  left-facing fighter's `air_slice` landed in the right place pointing right.
  Invisible on radial art, visibly wrong on slices, streaks and arrows.
  ⭐ **the pose comes out of the same expression as the offset, from the same two
  authorities** (the owner's frame and the move's committed facing), because a pose
  derived anywhere else can disagree with the position it decorates — one more row
  in a hand-kept reconstruction ledger is not a fix. The angle is the one the
  SPRITE renderer already stands a body up with, so a body and the effect hanging
  off it cannot disagree about which way is up. `FxPose` rides the event, the
  request and the message.
  ⭐ **identity is the default and every emitter states it OUT LOUD** — eleven
  construction sites had to answer, and a hazard saying upright is a fact rather
  than an omission. ⚠ unlike D149's swap this adds a TRANSFORM rather than a side
  effect, so it cannot double anything and needed no caller migration.

- ✔ **D140 — CLOSED 2026-08-16. A second match never started and never ended:
  "GO!" stayed up and nothing could win. (Jon, REPRODUCIBLE)** ⭐ the repro is a
  SEQUENCE, which is why every guard missed it — match ONE is correct in every
  shape, and every guard on this stage played exactly one match. His own *"I
  thought we had tests for that"* was the finding.
  **TWO defects that met on one `if`.** (1) `StocksMatchSettled` could not be
  RETRACTED between matches — it cleared only when `decide_stocks_match` observed
  no active match, and this stage never removes the receipt, so match two opened
  wearing match one's verdict. Retracted by ACTIVATION now: a match that is
  starting has not been decided. (2) the announce card had two writers and no
  arbitration — the GO! card holds one beat past the release and overwrote the
  victory card; the old guard protected the wrong half (*"do not CLEAR the
  winner's card"*), so once (1) made the verdict permanent, GO! sat on a live
  match forever. The ceremony now stops talking the moment the match is decided.
  ⭐ the product rule he stated is built: the sim clock is requested to `0.0` while
  a match is settled and `1.0` while one is live — self-healing, and safe because
  the sink reduces by `min` so hitstop still wins.
  ⛔ **the guard is the SEQUENCE — two matches in ONE app.** A test that builds a
  fresh app per match cannot fail this, which is why the existing ones passed.

- ✔ **D143 — CLOSED 2026-08-18. The stage's unarmed declaration reaches the seat;
  the publisher was reading its own deferred write. (found while answering Jon's
  moveset census)**

⇒ **what is left of this row is not plumbing.** Whether the peaceful cast should
be armed by the stage at all, or re-authored as fighters, is Jon's and is filed
in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). The
defect was real under either answer, and it is fixed under either answer.

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

✔✔ **INSTRUMENTED AND FIXED 2026-08-18 — and the cause was closer than the
guess.** The row supposed the declaration was installed with the GAMEPLAY route
and therefore absent at publish time. It is installed by **the same system**,
fifty lines below the read, through a DEFERRED `Commands::insert_resource`. So on
the frame the match is decided the resource does not exist. Measured on the
shipped select screen through its own taps:

```text
PROBE at publish: DeclaredCombatRules present = false, unarmed_melee = <no resource>
```

⇒ `smash_declared_combat_rules()` is the one source now: the publisher takes the
floor from the value it is about to declare, and inserts that same value. ⭐ and
reading the resource would have been wrong even once it existed — **on a second
visit it holds the PREVIOUS match's declaration**, a stale answer dressed as a
live one. A function has no such tense. ⚠ the `rules` system parameter is gone
with the read it existed for.

✔ **AND THE GUARD THAT COULD NOT FAIL NOW CAN.**
`the_match_gives_every_seat_a_kit_that_can_hit` spelled the swipe out by hand;
it calls `smash_declared_combat_rules().unarmed_melee` instead, so both sides
hold one copy. Poison-verified: with the stage's floor removed the test reports
the seat with no kit, which it could never do before.

⛔⛔ **AND A TEST THAT SEATS A PEACEFUL FIGHTER CANNOT BE WRITTEN TODAY** — I
tried, and it failed with *"fewer than two peaceful rows on the shipped grid"*.
D144 armed every selectable fighter, so **the floor has no subject**, exactly as
this row's own header says. ⇒ the guard is necessarily about the MECHANISM
rather than about a character, and that is a property of the roster, not a
weakness in the test.
⚠ **one false positive on the way, worth keeping**: filtering the grid by *"no
default melee"* selected `player_robot_v3`, which has an empty default action
set and sixteen AUTHORED moves — it arms itself by a different road, and the
publisher branches on exactly that. **"No default melee" is not "unarmed"**, and
a census that conflates them accuses the healthiest fighter on the grid.

⚠ **and the product question rides along, already filed**: whether the peaceful
cast should be armed by the stage at all or be re-authored as fighters is Jon's
(`awaiting-maintainer-decision.md`). This row is the PLUMBING half — the stage
says a thing and the body does not hear it — which is a defect under either
answer.

- ✔ **D144 — CLOSED 2026-08-16. Every selectable fighter now has the full
  sixteen-press smash kit** (robot v3 12→16, goblin and the Oni 11→16, the
  automaton 8→16, Mary-O, Sanic, Alice and Bob 0→16).
  ⭐⭐ **the up-B is the half that is not cosmetic** — the goblin, the Oni, the
  automaton, both protagonists and both Hall NPCs had NO special at all, which on a
  platform fighter is no way back to the stage.
  ⛔ **the census was wrong twice, and both errors are the lesson.** Asking whether
  a verb KEY exists reads a fallback as coverage (`directional_verb_chain` falls
  back, so a missing forward tilt is the jab again, not silence); and asking only
  ONE POSTURE invented a gap — George Booul's down-B is `airborne_only` by design,
  so probed standing it read as missing when he was 16/16 all along. **A press is
  covered when SOME posture reaches a move of its own**, and the census asks both.
  ⭐⭐ **a special owes an answer in BOTH postures** (Jon: *"a down-b that has
  special airborne properties should also have an effect on ground — think Bowser"*),
  and the mechanism was already in the engine with nothing using it:
  `directional_verb_chain` puts `special_air_down` ahead of `special_down`, so a
  two-form move is AUTHORED, not engineered. ⚠ the arcs are `ImpulseMode::Add`, not
  `Set` — `lift_speed` is derived from `Set` impulses, so written that way a hop
  told the recovery policy a DOWN-B was a way home. George's own up-B poison caught it.
  ⛔⛔ **it changes nothing in Mary-O's or Sanic's own games**: a move table is *what
  the swing IS*, the ability is *whether this body may swing at all*. That split is
  what makes "a platformer protagonist on a fighting grid" expressible.
  ⚠ `moveset_authoring` moved down to `ambition_characters` so a character from any
  provider can use it; ⛔ `ambition_demo_smash` still carries its own fork.
  ⭐ the census is a RATCHET (`report_the_smash_kit_every_selectable_fighter_has`),
  reading its target from `SMASH_KIT.len()` so trips and grabs joining the
  vocabulary raise the bar by themselves.
  ⛔ and one test was passing on a margin nobody had measured:
  `a_respawning_fighter_is_briefly_untouchable` called 300 `app.update()`s five
  seconds; the sim had advanced 1.73s. It failed because the app got heavier.

- ✔ **D145 — CLOSED 2026-08-16. No projectile could hit anybody on the smash
  stage, and it was never about the glider Jon happened to notice.** Melee and
  projectiles asked different questions about who may be hit, and only one knew
  what a match is: melee used `team_allows_damage(attacker_team, victim_team)`
  while the projectile loop used `damage_lands(firer_faction, victim_faction)` —
  no team, ever. Both seats come back `ActorFaction::Player` (correct: a Hall NPC
  and a demo protagonist are not enemies outside the match), so melee landed and
  **every shot from every fighter was spared as an ally**.
  ⭐ the fix is one call — `damage_lands_between` already existed and
  `StrikeVictim` had carried the victim's `team` all along, documented *"outranks
  faction for 'may this land'"*; this loop was the one caller that never asked.
  Guarded by a fixture with the poison inside it: a body on the FIRER'S OWN team,
  overlapping the same shot, that must not be hit.

- ✔ **D142 — CLOSED 2026-08-16. A match could only ever TAKE verbs away, so no
  stage could promise a fighter anything** (Jon: *"in smash all characters should
  be sure they are granted the basic smash abilities, but we want to do this in an
  elegant way"*).
  ⭐ **the elegant way is that a match has TWO things to say, not one** —
  `MatchAbilities { granted, permitted }`, with `effective = (authored ∪ granted) ∩
  permitted`. Smash declares `levelled(SMASH_FIGHTER_KIT)` (granted == permitted,
  one kit for everybody); versus declares `at_most(..)`, the lone mask it always was.
  ⭐⭐ **it makes a real tension EXPRESSIBLE instead of picking a winner.** Jon's
  older ruling is pinned by a test (*"forcing Puppy Slug into Smash gives you Puppy
  Slug — jump → no jump if its body cannot jump"*) and today's is its opposite:
  a match that MANUFACTURES capabilities is what made a slug jump like a humanoid,
  and a match that cannot GUARANTEE them is what sent the automaton onto a
  platform-fighter stage with no double jump. Two modes, two tests, one word at the
  stage. ⚠ the stated cost: a Puppy Slug forced onto the smash grid now jumps.
  ⚠ **one fighter changed in play** — the automaton gains double jump, fast fall,
  dodge, pogo and ledge grab. Its own row is deliberately NOT patched: a character
  does not carry verbs to compensate for a mode. Sanic authors `[RunJump]` on both
  iterations, replacing a `[SaneSubset]` that granted three of the four verbs Jon
  named it against; his super form is SPEED, not a capability unlock.

- ✔ **D141 — CLOSED 2026-08-16. One fighter on the smash grid could not grab a
  ledge, and the ledge lived at home on two who should not have it.** Measured
  across all fourteen: twelve author no kit and take the stage's set; of the two
  that author one, the Perfect Cellular Automaton's was written for the DUEL ARENA
  on `AbilitySet::basic()`, whose answer is `false` — and `fighter_abilities` was an
  INTERSECTION, so the stage could not give it back. **The one fighter whose sheet
  has ten ledge rows drawn for it was the one who could not use them.**
  ⭐ **the two roads are what makes "in smash but not at home" expressible**, and
  they are easy to mistake for one: a character's own game reads the catalog GRANT
  LIST; a smash seat reads the character DEFINITION ∩ the match's mask.
  ⛔ **a row that authors NOTHING falls through to `sandbox_all`** — Sanic was
  carrying ledge grab, swim, glide, dodge and a bubble shield around his own
  speedway, invisibly, because his control gate resolved Attack and Utility onto
  spin dash and transform. His row now authors `[SaneSubset]`, which excludes those
  five BY NAME. ⛔ what a runner's kit should actually be is still open.

- ✔ **D138 — CLOSED 2026-08-17, CONFIRMED BY JON: *"Oiler fights in his new body
  now."*** Both halves landed: the body swap (`69eee645f`) moved him off the
  Python-drawn sheet onto the direct-SVG rig, and the kit followed (`bd6cbf775` +
  `95b45b6cc`, sprite submodule `3f7d265`) — sixteen moves, eight new side-view rig
  clips, eighteen of his twenty-three effects bound, and the **geyser as his Up-B**
  (a commanded `Set` rise staging emerge → stream ×3 → impact). Both census
  ratchets moved in the same change: off `KNOWN_UNARMED` (8→7), onto
  `WITH_REPERTOIRE` (7→8).
  ⭐ **the architecture carried it with ONE new authoring primitive and no engine
  change** (`moveset_authoring::strike_tag`) and no character-ID branch anywhere.
  ⛔⛔ **A SHEET SWAP OWES THREE REGENERATIONS, NOT ONE.** After the rig's sheet was
  installed, `ultrapack.json` still carried Oiler at the TOON frame size — the tier
  atlases are baked from whatever was in `$sprites_dir` when they were last packed,
  so a target whose sheet changed keeps drawing its old art. Run `--target <t>`,
  then `regen_visual_quality_variants.sh --target <t>`, then the four ultrapack
  tiers. ⚠ this is very likely why a regen that DID replace the sheet still showed
  the old sprite.
  ⛔ **author arm poses as ANGLES on the reachable circle, never as (x, y)
  guesses.** Oiler's arms are 26.5px long from a shoulder 25.6px above the hanging
  wrist — already near full extension at rest — so every hand target picked by eye
  landed outside the circle, the IK clamped it, and all eight "big" swings collapsed
  onto the same 45° pose.
  ⚠ **remaining, both small**: four cues pack with a `.loop` suffix the sprite row
  does not carry, so the derived cue misses the bank
  — ⭐ **MEASURED 2026-08-18 and it is ONE live, THREE latent, and a DESIGN
  QUESTION rather than a fix.** `authored_effect(row).cue` derives the plain id
  for all four while the bank ships only the suffixed one:

```text
oil_geyser_stream -> vfx.oiler.oil_geyser_stream   packed .loop only   OVERRIDDEN ✔
invariant_loop    -> vfx.oiler.invariant_loop      packed .loop only   no move emits it
gate_calibration  -> vfx.oiler.gate_calibration    packed .loop only   no move emits it
portal_leak       -> vfx.oiler.portal_leak         packed .loop only   no move emits it
```

  ⇒ nothing is silent TODAY: the one live row already names its cue through
  `vfx_cued`, and the other three have zero Rust references. The trap is armed
  for whoever authors them — and for the other seven `.loop`-only rows
  `a_sustained_burst_keeps_its_looping_cue` counts.
  ⛔⛔ **and the two obvious fixes both cost something, which is why this is
  recorded rather than taken.** (a) a guard *"every derived cue must be one the
  bank ships"* fails on Oiler's own two geyser re-strikes, which emit a burst
  with no cue ON PURPOSE because the loop is still running — so it needs an
  exemption list, and an exemption list is a TODO list. (b) making the derivation
  fall back to `{cue}.loop` when the plain id is unshipped removes the trap
  entirely, and would retrigger the loop on each of those same re-strikes — the
  doubled-burst class `moveset_sound` exists to catch.
  ✔ **DECIDED 2026-08-19 — JON PICKED (c), LEAVE IT RECORDED.** Neither guard is
  built and neither fallback lands; a cue whose name does not follow the
  `{cue}.loop` convention still falls through silently and that stays a known
  footgun. ⭐ consistent with his standing rule that **an exemption list is a TODO
  list** — option (a) would have created exactly one. ⛔ this is decided, not
  deferred: do not propose the guard again unprompted.
  ⚠ the existing guard is the INVERSE of the gap: it checks bursts that already
  carry an override, never one that should and does not; the sheet has one upward and
  one downward swing, so tilt/smash share a row — honest, and thinner than the
  table. And a BALANCE pass with real eyes: he lost the observed match 36% to 5%,
  the design's direction but a bigger margin than intended.
  ⚠ **why this row was stale for a day**: `regen_sprites.sh` still listed `oiler`
  in `review_cues`, which would have overwritten the rig's sheet on every full run.
  That entry is now an explicit ⛔ refusal pointing at `tackon_targets`.

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
**intentionally resettable** ⇒ may recreate (already the default).
⭐ **verified 2026-08-18: still ZERO producers**, and the three notes that say so
(`continuity.rs` twice, `rollback/domains/primitives.rs` once) are accurate — the
one production mention is a match ARM that treats `Consumed` as terminal, not a
write. ⇒ this leg is a FEATURE waiting on a product answer (what destroys
something permanently?), not a defect. Plus two
carried risks the author recorded: `SimId::placement(id)` is a **global**
namespace whose uniqueness is only checked **per room**, so two rooms authoring
one id would suppress both; and the ledger is not experience-scoped, so a
suppressed row can survive into a new session.

✔ **THE FIRST OF THOSE IS GUARDED NOW (2026-08-18), while it is still free.**
`validate.placement_id_collision` warns when one authored `id` names things in
two rooms. ⭐ **green on every shipped world, which is exactly why it was worth
adding today** — measured: twelve entity kinds carry an `id`, and the ONLY
cross-room reuse is `LoadingZone` (`return_door` in 7 rooms, `east_exit` in 3,
`west_exit` in 2), every one deliberate because a zone's `target_zone` resolves
within its `target_room`. Nothing else collides, so the guard costs nothing until
it earns its keep.

⛔⛔ **and the collision is REACHABLE, not hypothetical**: `authored_logic/
prepared.rs` turns an authored `placement:<id>` argument into
`SimId::placement(..)`, and that is production. No authored content names one
yet — so the day the first rule says `placement:return_door` it would mean seven
zones, and this now says so first. ⚠ four tests pin it, because a guard that is
green on all real data is otherwise indistinguishable from one that cannot fire.
⚠ per FILE: a cross-WORLD collision is possible in principle (measured 0), and
checking it would need every world loaded at once, which this validator does not
do.

▢ **THE SECOND CARRIED RISK IS STRUCTURALLY CONFIRMED (2026-08-18) AND NOT YET
FIXED, because the fix is a boundary question rather than a line.**
`AuthoredOccurrences` and `OccurrenceBaseline` are plain global `Resource`s
(`init_resource` in `items/pickup` and `checkpoint_horizon`), and **no experience
scope names either**: the shell's scopes release `MatchParticipantRoster`,
`DeclaredCombatRules`, `PreparedMatch` and a handful of smash-local values, and
nothing else. ⇒ a suppression written in one experience is still there in the
next.

⭐ **`ExperienceScopeBuilder::resetting` is the right KIND** — *"put back to its
default. Never a removal, so it makes no ownership claim"* — which is exactly the
semantics wanted: a new session starts with a clean ledger, and no game is
claiming to own the resource.

⛔⛔ **but WHERE it is declared is the whole question, and the wrong answer is an
exemption list.** Scopes are authored per experience by each game, so adding
`.resetting::<AuthoredOccurrences>()` to the two that exist today makes the third
game's omission a silent bug — the shape D152 already solved once and D136
catalogues: **the ENGINE owns the invariant, the composition owns the order.**
There is no engine-side hook to contribute a release to every scope; that seam is
the work.

⭐⭐ **TRACED TO ONE BOOLEAN, AND IT IS THE SAME BUG THEY ALREADY FIXED ONE
LEVEL DOWN.** The ledger's restore is not the problem — `adopt_rows` REPLACES
(`self.rows = rows`), so an empty save correctly clears it. What survives is the
GATE:

```text
restore_durable_horizon   returns early on `SaveRestored`
SaveRestored              set true in `restore_inventory_from_save`, and set
                          false NOWHERE — one write in the tree, `restored.0 = true`
                          ⇒ experience 2 in a process never re-runs the restore and
                            inherits experience 1's AuthoredOccurrences,
                            OccurrenceBaseline, CustodyBaseline, MintedItemBaseline
```

⛔⛔ **and the latch's own doc already states the rule it breaks**: *"the flag
now means 'the loaded save has been applied to THIS WORLD'"*. A new experience is
a new world. ⭐ **they fixed exactly this for ROLLBACK on 2026-08-04** — *"a
rewind past the restore undid its EFFECT and kept the record of having applied
it"* — and registered the latch as rollback state so it rewinds with what it
guards. The experience boundary is the same sentence with a different clock.

✔✔ **FIXED 2026-08-18, and neither of the shapes I costed was needed — the
codebase had already designated the place.** `session::teardown`'s
`reset_session_scoped_resources_on_retire` reads `SessionScopeRetired` and
already clears TEN process-global mirrors, with exactly this rationale: *"the
mirrors are process-global rather than per-scope, so a single reset on retirement
is correct"*. `SaveRestored` is that, and it is now the eleventh.

```text
RouteActivated    → active_scope.begin()          ⇒ a new experience DOES mint a scope
RouteDeactivated  → GameMode, Dialogue, RoomTransition, Cutscene all reset,
                    "every one of them describes a live world, and this one has
                     just been retired"           ⇒ SaveRestored belongs in that list
```

⭐ **one value fixes all four ledgers**, because `adopt_rows` REPLACES rather
than merges: the next session's restore rewrites `AuthoredOccurrences`,
`OccurrenceBaseline`, `CustodyBaseline` and `MintedItemBaseline` from the save,
empty or not. ⭐ **no schema bump** (the latch's shape is unchanged, so its
rollback registration is untouched) and **no new hook** — the two options I had
costed, keying it on the session generation or adding a scope-watcher, were both
more machinery than the seam that already existed.

⇒ **and the shell system says why it belongs there rather than in a game's
scope**, in Jon's own framing: *"A rule every caller must obey is a rule three
callers will eventually break. The lifecycle that ended the session is the one
place that cannot forget."*

⚠ guarded by `retirement_clears_the_save_applied_latch`, which asserts BOTH terms
— the latch survives an ordinary frame (clearing it mid-session would re-apply
the save over live state) and is cleared on retirement. Falsified: removing the
one line fails it by name.

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
(`a_banked_object_whose_room_unloaded_returns_to_the_hand_that_banked_it` (renamed `13dd4d31b`)). A baseline row
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
✔✔ **ANSWERED 2026-08-18 with the registration assertion it asked for.** The
behaviour test lists `return_released_items` in a chain of its own, so deleting
the production registration left it green. `the_production_plugin_registers_the_custody_release`
builds `ItemPickupSimulationPlugin` and asks the SIM SCHEDULE whether the system
is in it. Poison-verified by deleting the real registration:

```text
an_item_stowed_from_the_menu_…_can_be_taken_again   ok       <- the behaviour test
the_production_plugin_registers_the_custody_release  FAILED   <- the new guard
```

⛔ **and the first draft of the guard was itself the bug it hunts**: it
initialized the schedule against a FRESH `World`, enumerated zero systems, and
reported "not registered" for a system that is. It now initializes against the
app's own world and asserts a non-empty floor first, so "the enumeration is
broken" can never be read as "the registration is missing". ⇒ every count needs
a zero floor.

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
fighter's — and not a bug with an obvious fix. ▢ **now asked, as §15 in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md)** — it had
been "recorded as a fork" in this row alone since 2026-08-17, which is where
questions go to wait for nobody. ⛔ do not guess it.

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

⚠⚠ **AND THE GAIN IS NOT PROTECTED YET, which is a consequence worth stating.**
A ratchet locks a win by being RE-FROZEN, and `compile_ratchet.py --update`
writes the WHOLE snapshot — so banking the 497 lines would also bank
`ambition_geometry`'s and `ambition_platformer2d_core`'s edit-cost regressions,
which are eight days of unrelated growth nobody has accounted for.

⚠⚠ **AND IT HAS DRIFTED — MEASURED AT HEAD 2026-08-18, one day later.** The
same count that produces the ledger's own 110,932 at `355874fe1` gives:

```text
355874fe1   110,932   the recorded win, UNDER the 111,429 baseline
HEAD        115,562   +4,630 in a day, and +4,133 OVER the baseline
```

⛔ **but a carve is the WRONG response, because the biggest single item is debt
that moved here BY DESIGN.** Where it went:

```text
+1,555  features/ecs/capture.rs (NEW)   the grab campaign — a whole mechanic
  +767  rollback_registration.rs (NEW)  the domain-owned rollback merge
  +589  world/authored_switch_commands{,/tests}.rs (NEW)   D136's own inversion
  rest  tests and ordinary feature growth
```

⭐ the +767 is a RELOCATION, and the other side of it is visible in the same
window: `ambition_platformer2d_runtime` went **17,652 → 15,554 (−2,098)** as the
`rollback/domains/*` files dissolved into the crates that own the state. That is
this repo's own rule working — *"the destination joins in the same commit"* — and
a per-crate line ratchet reads it as the monolith rotting.

⇒ **so the honest statement is not "the monolith grew 4,133 over budget"**; it is
that one mechanic (+1,555), one inversion this row asked for (+589) and one
ownership transfer that shrank a sibling by 2,098 (+767) account for most of it.
⛔ a session that reads only the total will carve something to pay for work that
was correct.

⇒ **it stays unfrozen on purpose, and the monolith may drift back up to
111,429 for free until the other findings are dealt with.** ⛔ do not re-freeze
to make the tool quiet — that is the laundering this row already paid for once.
⭐ the honest sequence is: account for the edit-cost regressions FIRST, then
re-freeze everything together and the size gain locks in with them.

⚠⚠ **AND IT DID DRIFT BACK — MEASURED 2026-08-18, ONE DAY LATER, AND PAST THE
BASELINE RATHER THAN UP TO IT:**

```text
111,429   frozen baseline (2026-08-09)
110,932   four modules relocated  355874fe1   ⭐ UNDER, for one day
112,357   2026-08-18                          ⚠ +1,425 back, +928 OVER baseline
```

⛔ **and the ratchet says NOTHING about it**, because `largest_unit_lines`
carries a +2,228 growth budget and 112,357 sits inside it. ⇒ **the win was never
protected and the instrument was never going to report its loss** — which is a
sharper statement of this row's own warning than the warning was.

⭐ **where the 1,425 went, so nobody has to re-derive it:**

```text
+552  world/authored_switch_commands   D127 M2's authored-rule work
+433  features/ecs
+117  character_runtime/prepared_match
+115  time/time_control                a guard test (D117)
 +83  world_facts.rs
 +86  session/teardown + control/input_systems   D125's latch fix + its note
```

✔ **AND THE REPORT NOW SAYS IT** — `largest_unit_lines` prints
`[frozen 111,429, +928, budget ±2,228 within budget]`. ⛔ **the GATE is
unchanged**: same 8 findings, same exit code. A budget answers *"is this worth
failing on"*; it does not answer *"are we where we thought we were"*, and only
the second question was unasked. ⛔⛔ **deliberately NOT a tightened budget** —
Jon's ruling stands (*"the compile ratchet is an INSTRUMENT, NOT A TARGET"*) and
a tighter budget makes it more of one. Five tests pin it, including that a
DIFFERENT crate taking the title is flagged rather than compared as one number.

⭐⭐ **the largest contributor is NOT a violation of this row's standing rule,
and reading the destination's contract is what settled it.**
`authored_switch_commands` is a runtime interpreter of authored world IR, and
`ambition_platformer2d_world` opens with *"Backend-agnostic authored world IR …
simulation crates interpret them through explicit lowering seams"* — an IR crate
refusing an interpreter in its own words. ⇒ same move as the four relocations'
three refusals: the contract turned a plausible destination into an obviously
wrong one at zero cost.

⭐⭐ **AND THAT ACCOUNTING IS NOW HALF DONE — `--diff` costs seconds and runs no
build.** The edit-cost regressions split cleanly into two causes, and only one
of them is this row's doing:

```text
crate                    +dependents   +lines     +seconds
ambition_geometry              +0     +60,697     +257.8s   ← workspace GROWTH
ambition_asset_manager         +0     +61,964     +292.0s   ← workspace GROWTH
ambition_encounter            +17     +34,969     +218.6s   ← STRUCTURAL
ambition_dialog               +12     +30,649     +206.0s   ← STRUCTURAL
ambition_platformer2d         +12     +18,541     +128.1s   ← STRUCTURAL
ambition_touch_input          +12     +18,553     +128.1s   ← STRUCTURAL
```

⭐ **`+0 dependents` means nobody new depends on it — its closure simply got
bigger**, i.e. ~61k lines of ordinary feature growth landed above it in eight
days. The monolith's −10,890 today sits INSIDE that, which is why the workspace
total is up while this crate is down.

⛔ **the `+N dependents` rows are the carves' own bill.** Relocating and carving
gives the destination new dependents, so the crate BELOW gets more expensive to
edit even as the monolith gets cheaper — the inverse of the laundering trap, and
the reason a per-crate ledger cannot score this row on its own.

✔✔ **ANSWERED 2026-08-17, AND THE ANSWER REFUSES THE QUESTION'S PREMISE.** Jon,
verbatim: *"like count is a proxy, decompose as it makes sense. try not to dump
things into it to make the problem worse."*
⇒ ⛔⛔ **the compile ratchet is an INSTRUMENT, NOT A TARGET.** Do not schedule a
carve to move a number, and do not re-freeze to make a tool quiet. The 17
dependents were never the real question — carve where OWNERSHIP says so, and if
the number happens to fall, good.
⭐ **the operative half is the second clause, and it is a STANDING rule that
binds every other row in this ledger, not just D33: new work does not land in
`ambition_platformer2d_actor_monolith` because that is where its neighbours
already are.** A feature whose owner is elsewhere goes to its owner even when the
monolith is the cheaper edit. That is what stops this row needing to exist again.

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
`SettingsOutcome`, `DevToggleSnapshot`, `apply_action`, `apply_display_mode` (⚠ GONE — left the monolith at `355874fe1`; the logic is `ambition_settings_menu::settings::apply` now),
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

⭐⭐ **THE `audio`+`music` CANDIDATE IS MEASURED, 2026-08-18 — AND IT NEEDS NO NEW
CRATE, because the destination already exists and its contract ACCEPTS it.**

```text
1,842 lines   972 production, 870 tests
    3         outward `use crate::` statements to the rest of the monolith
    1         of those is a GENUINE edge — and it is test-only
```

The three, chased to their definitions rather than counted:

```text
crate::rooms::RoomMusicRequest             → ambition_platformer2d_world   RE-EXPORT, below
crate::assets::game_assets::GameAssetConfig → ambition_sprite_sheet        RE-EXPORT, below (test)
crate::session::data::{MusicRegistry,…}    → ambition_audio               RE-EXPORT, below
crate::session::data::{fixture_*_registry} → session/data.rs:51,57        ⚠ REAL, pub(crate), TEST
```

⇒ **one real edge, `pub(crate)`, and only tests use it** — so the carve's actual
cost is finding a home for two fixture builders, not untangling a dependency.

⭐⭐ **AND THE DESTINATION IS `ambition_audio`, WHICH ALREADY OWNS THE PARTS.**
It opens *"Content-free audio data/runtime layer"* and already ships `library`,
`render`, `music`, `mix`, `web_unlock` plus three Bevy plugins. The monolith's
`audio/plugin.rs` (422 lines) is largely a COMPOSITION of that crate: it installs
three `ambition_audio` plugins and initialises eight `ambition_audio` resources.
Monolith-specific state is four items — `RadioStationState`, `AudioEnvironment`,
`DefaultMusicStarted`, `MusicIntent`. ⇒ same shape as the `boss_encounter`
relocation: an existing owner, no new crate, no new hop.

⛔⛔ **AND A STALE DOC COMMENT NEARLY REFUSED THE CARVE FOR THE WRONG REASON.**
`music/mod.rs` describes itself as carrying *"authored goblin cue data"*. Under
the read-the-destination's-contract rule that reads as an instant refusal —
content-free crate, authored content, done. ⚠ **it is not true any more.**
Grepping the two modules for named game content finds ZERO string ids naming a
track, boss, room, or character; the only tuning value is one
`LARGE_BRUTE_DELAY_SECONDS = 3.5`. The goblin cue left long ago and lives at
`game/ambition_content/src/music.rs` (`FIRST_GOBLIN_CUE_ID`,
`MOB_LAB_ENCOUNTER_ID`). ⇒ **the destination's contract must be read against the
CODE's present content, not the source module's description of itself** — a
stale self-description is a refusal the code no longer earns, which is the
inverse of the failure this row usually catalogues and costs just as much.

⛔⛔ **AND THE `use crate::` COUNT DID NOT PRICE THE MOVE — the EXTERNAL crate
deps did, and they split the candidate in two.** Three outward intra-crate edges
looked like a free relocation. What actually costs is what each module imports
from OTHER crates, because those become the destination's new dependencies:

```text
audio/environment.rs   bevy · bevy_kira_audio · ambition_audio::library        ← nothing else
audio/plugin.rs        + ambition_platformer2d_shared_tangle · ambition_dev_tools
music/intent.rs        + ambition_encounter · ambition_platformer2d_world
```

⇒ **moving the whole 1,842 lines would make `ambition_audio` — today a leaf on
`ambition_sfx`, bevy, bevy_kira_audio, ron, serde — depend on `ambition_encounter`
and `ambition_platformer2d_world`.** No cycle (checked: none of the five names
`ambition_audio` back), but a foundation acquiring mid-level dependencies is the
carve making the graph worse, which is exactly what Jon's *"try not to dump
things into it"* rules out.

⭐ **so the shippable slice is `audio/environment.rs` ALONE** — 238 production +
181 test lines, realtime channel-attenuation DSP whose only outside import is
`ambition_audio::library::{amplitude_to_decibels, MusicChannel, SfxChannel}`.
It already reaches INTO the destination for every type it uses; moving it adds
the destination not one new dependency. `plugin.rs` and `music/` stay until
someone wants a music-direction crate ABOVE `ambition_encounter`.

⭐ **the transferable half: an intra-crate `use crate::` census answers "what
would break", and the EXTERNAL import census answers "what would the destination
inherit".** Only the second one prices a crate boundary, and this candidate looks
free by the first measure and expensive by the second.

⚠ still not measured: whether `ambition_audio` can carry the monolith's
`audio`/`web_audio` persona features (it has `kira`), and the five-lockfile /
contracts-job bill every crate-boundary change here has paid.

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
  World`. `load_room` (24 params), `apply_room_transition_resets` (⚠ GONE — folded into the one room-transition application at `a2b6652e7`, D71),
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

- ⏸ **D64 — Mary-O / LDtk authoring. RESTING as a successful ACCEPTANCE BASELINE,
  not a running campaign (2026-08-15).** A new level can be created through LDtk
  **without adding ordinary Rust level registration**: authored rooms need no Rust
  routing to exist, destinations and warp tubes are authored, one shared
  `ldtk_entity_contract.json` makes the Rust prover and the Python validator refuse
  exactly what the real converter refuses, and a ratchet guards the level roster.
  That is an Engine 1.0 milestone.
  ⛔ **do not keep adding Mary-O tooling because the lane existed.** The next LDtk
  improvement must come from actual content-authoring friction: author a room, hit
  a real limitation, fix *that* generically.
  Preserved rules: `.ldtk` is the authoritative spatial source · tools edit it
  additively and in place · destructive bootstrap regeneration must not return ·
  Rust and Python validation must agree · game-specific semantics stay
  provider-owned rather than growing a central engine taxonomy.
  ⛔⛔ **I filed lives as unstarted and it had already landed** — I wrote the row
  from a `▢` in Jon's observations file without grepping HEAD. **A marker in a
  maintainer's file is a REPORT, not a measurement.** What was actually wrong was
  three doc claims that outlived the behaviour they described.
  ⛔ **the Mary-O presentation guards do not run in the ordinary suite**
  (`#![cfg(feature = "visible")]`: 36 tests bare, 44 with it), which is why a
  maintainer row about a hidden block read as unfixed when it had passed all along.
  Run the `visible` suite before and after any Mary-O visual work.
  ⚠ **and the hole is the whole workspace's, measured 2026-08-14**: 24 crates hide
  **629 tests** behind features, and there is no automatic runner —
  `.github/workflows/test.yml` is `workflow_dispatch` only and the per-turn gate is
  one integration target, both deliberate. ⇒ what runs is a MAINTAINER decision;
  ⛔ do not enlarge `gate_suite.py`, and ⛔ do not add a job to a workflow that does
  not run and call the hole closed.

---

## Waiting on an external fact or maintainer decision

These are real unresolved items but are deliberately **not** `▢` queue work.
⭐ **a `✔` row here is one Jon has since answered**, kept in place for one pass so
anyone who came looking for the question finds the ruling instead of a gap.

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
- ✔✔ **D114 — CLOSED 2026-08-17 BY MAINTAINER RULING. Hitlag freezes the BODY that
  is in it, on both roads, and the old per-body-zero-dt prohibition is
  SUPERSEDED.** `818218949` gave the actor road
  `let sim_dt = if combat.is_in_hitlag() { 0.0 } else { dt };`, so a hit between
  two actors freezes both — which it never did, and CPU-versus-CPU froze nobody.
  ⭐⭐ Jon kept it and overruled the prohibition: *"hitlag is a combat/body
  semantic, not something that should depend on whether a body happens to occupy
  the primary local-control road."* ⛔⛔ **the three options this row used to offer
  are void — every one of them preserved a per-road distinction, and the
  distinction WAS the defect. If hitlag ever feels too sticky, tune its DURATION
  or SHAPE; restoring a controlled-body/actor asymmetry is forbidden.** ⚠ the
  superseded warning was not wrong when written — it was measured before D155, on
  a build where nobody was ever launched, so **a feel verdict inherits the build
  it was formed on**. Ruling in
  [`maintainer-decisions.md`](maintainer-decisions.md); record in
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §6.

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

- ▢ **D127 — Deterministic authored gameplay logic and orchestration. M0 COMPLETE; M1 MET FOR BOTH HALVES; M2's PREPARED-CALL half landed 2026-08-17; the `when … then` RULE FORM is deliberately absent for want of a customer.**

⭐⭐ **ACTIVE TRUTH — read this paragraph and stop; everything below it is
EVIDENCE, in reverse-chronological layers, and several of its older `⇒ NEXT` /
`⚠ STILL MISSING` claims were true when written and are not true now.** Checked
against `7e7552c4b` on 2026-08-17:

```text
M0  ✔ complete
M1  ✔ met for BOTH halves — conditions AND commands have a domain-owned
       provider contract (PublishCondition / PublishCommand on App, private
       catalogs, no central enum edited to add a provider)
M2  ◐ the PREPARED CALL landed: PreparedCondition / PreparedCommand, private
       fields, no public constructor, id+arity+kind validated at prepare time,
       authored text NOT retained, an authored reference prepared into SimId.
       ⛔ the generic `when … then` container was DESIGNED AND CUT on purpose —
       zero adopters, so it would be falsifier 2's wrapper. It needs a REAL
       CUSTOMER before it is built; do not "finish the rule form".
M5  ▢ diagnostics, untouched
```

⇒ **two follow-ups are NAMED and are not this row's next action** (neither is
cleanup work, and neither was done deliberately): `gated_lock_walls` still
rebuilds its condition arguments every tick instead of holding a
`PreparedCondition`, and `ambition_conversation::dialog::authored_commands` still
owns a second text→`AuthoredArg` conversion that `prepared::prepare_args` now
generalises.

⭐⭐ **M2 — ACCEPTANCE MET FOR ONE CALL, NOT FOR A `when … then` RULE, and the
difference is deliberate.** `authored_logic::prepared` turns one authored line
(`<id> <arg>…`) into a `PreparedCondition` / `PreparedCommand`: private fields,
**no public constructor**, so the only door is a `prepare` that checks the id
against the published catalog, the arity against the descriptor and every value
against its declared kind. All four acceptance clauses are structural rather than
promised — validation cannot be skipped (unconstructible otherwise); the runtime
parses nothing (the prepared value does not RETAIN the text); the data is
immutable (no `&mut` accessor exists at all); and a reference is a `SimId` minted
through `SimId::encounter` / `SimId::placement` from an authored
`encounter:<id>` — never `from_snapshot`. ⭐ **that is the refusal both M1 Yarn
bridges recorded as "a thing M2 can replace", replaced.**

⭐ **the deletion:** `KERNEL_FACES`'s `(switch id → signal key)` half and the
`SwitchActivated` loop in `ambition_content::encounters` are gone. The four
kernel switches author `on_activate: encounter.signal encounter:symmetry_attunement gravity_down`
in `symmetry_room`, prepared once per room by `world::authored_switch_commands`
and performed by `encounter.signal`, this contract's second command provider.
⚠ **the spawn-side half survives with cause** — `KERNEL_SIGNALS` builds the
encounter's own `Objective::All`, which is a puzzle stating its win condition
rather than a table saying which switch does what. The end-to-end
`symmetry_attunement` app fixture passes UNCHANGED, driven entirely by the level.

⛔ **NO PROGRAM COUNTER APPEARED, and that is the answer to M0 Finding 4 for this
slice**: a prepared call is one call, evaluated fresh, with nothing to rewind.
The `when` half of a rule was designed and CUT — the one customer's condition
list is empty in all four rows, and a shipped `when` with zero adopters is
falsifier 2's wrapper. ⇒ **M2 is met for the representation; the rule FORM is
still open and needs a customer before it is built.**

⚠ **two follow-ups this slice named and did not do**, both small and both
recorded so they are decisions rather than drift: `gated_lock_walls` still
rebuilds its condition call from a `String` every tick instead of holding a
`PreparedCondition` (its validation is still per-tick), and
`ambition_conversation::dialog::authored_commands` still owns a second copy of
the text→`AuthoredArg` conversion that `prepared::prepare_args` now generalises —
a fork worth collapsing, which would also give authored `.yarn` prepared
references for free.

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
asserts exact arity so the verb takes exactly one argument (stands, and it is
FUNCTION-only), and `ParamKind::Reference` was refused rather than coerced from a
quoted string — ✔ **that refusal is discharged: M2 prepares a reference into a
real `SimId`.**

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
argument was **REFUSED with a reason** rather than coerced from a quoted string:
⭐ *a `.yarn` literal is not an identity, and answering confidently about
whichever occurrence happens to share the spelling is worse than not answering.*
✔ **the SECOND limit is DISCHARGED (2026-08-17), and the refusal was the right
placeholder**: M2's `prepare` mints a reference as a real `SimId` through
`SimId::encounter` / `SimId::placement` from an authored `encounter:<id>`, so a
reference is now an identity rather than a spelling. ⚠ the arity limit stands and
is FUNCTION-only — a Yarn *command* is dispatched by name with a parameter list
and takes as many arguments as its descriptor declares.

◻ **HISTORY (2026-08-16, SUPERSEDED THE NEXT DAY by the command half below — do
not work it): COMMANDS HAD NO PROVIDER CONTRACT.** Conclusive grep at the time — no `PublishCommand`, no command
catalog, nothing. ⭐ kept because **the reasoning is what SHAPED the contract that
landed**, not because anything here is outstanding: a condition is a
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

✔✔ **AND THE COMMAND HALF LANDED, 2026-08-17.**
`shared_tangle::authored_logic::commands` mirrors the condition contract with the
same privacy and one command published: `world.set_flag(flag, on)`, from
`world_facts.rs`, beside its condition twin.

```text
CommandCatalog     PRIVATE `publish` — the rollback waiver, reproduced first
                   PRIVATE `run`     — the AUTHORITY answer
RunAuthoredCommand a Message: the only public road to `run`
AuthoredCommandSet inside GameplaySimulationRoot, after CoreSimulation,
                   before GameplayEffects — both sets in the SAME schedule,
                   so neither pin is the vacuous cross-schedule kind
AuthoredArg        ⭐ the RENAMED `ConditionArg`, shared by both halves rather
                   than forked into a `CommandArg` with the same four variants
CommandOutcome     Done | Refused(reason) — one FEWER answer than a condition's,
                   because "I cannot tell" is a question's answer, not a verb's
```

⭐⭐ **AUTHORITY, the one thing with no precedent to copy: it is the RUNNER, not
the requester.** `run` is private, so holding the catalog lets a caller DISCOVER
the vocabulary and speak none of it; the only reader of `RunAuthoredCommand` is
`run_requested_authored_commands`, in the same file. ⇒ anything that can write a
message can ASK; nothing at all can perform a verb out of phase. A per-command
list of permitted callers was rejected — this engine has no vocabulary for *who*
a caller is that is not already a seat, a session or a body.

⭐ **ROLLBACK, answered by construction rather than by argument.** The runner
writes `SetFlagRequested` — a channel that already existed, is already cleared on
rollback, and is already applied by `apply_flag_effects` in the phase the set is
ordered before. **The command introduces no new kind of write**, and it keeps the
quest mirror the effect bus does on every flag write. ⚠ the dispatcher DRAINS
rather than reading with a cursor, so it holds no `Local` — the trap a
message-clear usually closes is structurally absent, and the one registration
covers only the residual window (a request released onto a frame the host rewinds
past before the set ran). Wire format 358 → 359 stable names, encoded types
unchanged at 85, schema v35 → v36. ⚠ the JSON baseline's count field read 357
against a 358-entry list beforehand — stale, corrected in passing.

◻ **HISTORY (2026-08-16, and BOTH of its prerequisites were PAID on 2026-08-17 —
`KERNEL_FACES`'s pairing half is DELETED; see the M2 block at the top of this
row): THE NAMED DELETION GATE WAS REFUSED, WITH CAUSE.**
It pairs an authored switch id with a *signal key*, which is the ENCOUNTER
domain's vocabulary. Deleting it needed (1) a second command,
`encounter.signal(<encounter>, <signal>)`, and — decisively — (2) **an authored
LDtk surface that carries a command WITH ITS ARGUMENTS**, which is *prepared
arguments from authored source* and is **M2's job by this plan's own
assignment**. ⚠ the condition half hit the same wall and NARROWED rather than
invented: `LockWall.gated_by` names a flag, not a whole condition, because *"an
authored surface is much harder to take back than to widen."* ⚠ and only half of
`KERNEL_FACES` is even the offending shape — its use in the spawn builds
`Objective::All([ReceiveSignal(…)])`, an encounter stating its own win condition,
which survives any version of this.

⭐ **A REAL DELETION WAS PAID INSTEAD, in the same file the condition half paid
its second one in.** `ambition_content::yarn_vocabulary` lost `cmd_set_flag` and
`cmd_clear_flag` — two hand-written Bevy systems differing by one bool, each
registered by name in a second list, each with its own conversion from Yarn's
untyped text — plus both `add_command` lines, the header's classification row,
and `NarrativeInputPlugin::<SetFlagRequested>` (whose only narrative writer they
were). `intro.yarn` and `kernel.yarn` now spell it
`<<command "world.set_flag" "<id>" true>>`.

⇒ **THE RIVAL MECHANISM IS NOW GONE IN BOTH DIRECTIONS.** Authored content asks
with `condition("world.flag_set", …)` and tells with
`command("world.set_flag", …)`, and a domain publishing either adds nothing to
any bridge, vocabulary table or game crate.

⭐ **and the command verb is NOT limited to one argument, which turned out to be
a FUNCTION-only constraint.** Yarn's VM asserts a function call's argument count;
a command is dispatched by name with its parameters as a list, no assertion, and
`Option` params retrieve `None` when the list runs out. So `world.set_flag` takes
two arguments and `set_flag`/`clear_flag` collapsed into ONE verb. ⚠ but every
authored argument arrives as TEXT (Yarn types a function's args, not a command's),
so the bridge parses against the published descriptor's kind — and `Truth` accepts
exactly `true`/`false`, because a lenient parse maps an unrecognised spelling to
`false`, and `false` on `set_flag` CLEARS.

✔ **M2's PREPARED-CALL HALF LANDED (2026-08-17) AND THE REFUSAL ABOVE WAS ITS
CUSTOMER**: prepared arguments from an authored source, so an LDtk switch now
carries a command call (`on_activate: encounter.signal …`) the way a `LockWall`
carries a flag. ⛔ **what is NOT next is the `when … then` rule form** — read the
ACTIVE TRUTH block at the top of this row.

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
  sentence about the wrong half of a value**. Fixed by deleting
  `GatePortalConfig.phase` and registering a `GatePortalPhases` resource **with a
  value projection**, because a presence-only probe would have passed while
  reproducing the defect. Schema **31**. ⭐ the generalisable shape: *"a registered
  input with an unregistered integral."*
- ✔ **the `Brain` cursor's `_` arm — MOSTLY DISSOLVED.** Six brain families looked
  like they failed to rewind; `rollback_component_cursor` clone-snapshots the whole
  component, so they rewind fine and what they lack is **desync DETECTION**.
- ⭐ doctrine correction landed with these, because it was being misread: **a
  central *authoritative* census that every new domain must edit is bad; a derived
  *read-only* discovery index that domains contribute descriptors to is good and is
  required.** ⛔⛔ do not sacrifice discoverability in the name of avoiding central
  authority. Recorded in `simulation-authority-and-determinism.md` and
  `inspection-diagnostics-and-workbench.md`.

- ✔ **D133 — CLOSED 2026-08-16. The durable save horizon: what the world remembers
  about occurrences now survives closing the program.**
  ⭐⭐ **the on-disk form IS the checkpoint's own description, serialized** — not a
  fourth description of the same facts. `AmbitionGameSaveData` gained three
  `#[serde(default)]` lists that are `AuthoredOccurrences`, `CustodyBaseline` and
  `MintedItemBaseline` field for field (save version 3 → 4), **and that the file
  needed no field the checkpoint slice had not already measured is the finding**.
  ⭐⭐ **a load is a checkpoint RESUME** — adopt the ledger and the three baselines,
  write one `ResetToCheckpoint`, and the road a death already takes rebuilds the
  world. Two systems, no new reconstruction logic, still exactly one authority.
  ⛔⛔ **a defect the fixture found that nothing else could have**: a session builds
  its start room before any file is read, so `record_placed_ground_items`
  republished the stale position over the loaded row — sending the object home and
  resurrecting a terminal row. ⭐ the fix is an INVARIANT: an occurrence comes to
  rest here only if its row says `InCustody` or already says `Placed` here, because
  **an object cannot change rooms without being carried.** It refuses rather than
  repairs, so it is not a second reconstruction authority.
  ⚠ **schema 33 → 34 for a RENAME and nothing else**; ⭐ the save file is not
  rollback state — the three values it serializes were already registered.
  ⛔⛔ **PROMOTED TO A PREREQUISITE 2026-08-17**: a runtime mint NOT in a hand at
  save time (lying in a room, in flight) is undescribed and lost, because the
  description remembers no position — **and Jon's dropped-weapon ruling makes that
  exact case a product requirement** (a unique weapon stays where it fell). So this
  is the blocking item, not a noted residue. Also still open: `Consumed` round-trips and still has no live producer;
  `load_save_at_startup` is presentation-only, so a headless composition never
  writes a file; and the body resumes at the shrine while the objects resume at the
  autosave's instant.

- ▣ **D132 — HALF CLOSED 2026-08-16. The same item had two persistence authorities
  and they had never been asked to agree.** ⭐⭐ **measured first, and the
  prediction was wrong about which history breaks**: the save/load/mint/bank/die
  scenario ends with the player holding it AND owning it, the count decremented at
  no beat, and the second round-trip agreeing with the hand **by coincidence rather
  than by rule** — but that history is not the defect.
  ⛔ **the defect was next door: `OwnedItems` was not checkpoint state at all.** A
  pressed pickup `grant`ed a catalog row beside taking custody, so one acquisition
  left TWO records and only the object's rewound — acquire after the checkpoint,
  die, and the menu equips a phantom whose throw mints a SECOND real weapon.
  ⭐ **closed by DELETION**, both halves probe-falsified: the `grant` is gone (the
  object is the record) and `OwnedItems::count` PROJECTS the equipped slot, so the
  grid loses it exactly when the hand does. No schema change.
  ▢ **STILL OPEN, and the gate is named**: a quantity conferred by
  `<<give_item>>`/shop/drop keeps its row through the mint, so it can still
  manifest a second object. ⇒ `OwnedItems` must join the checkpoint baseline first,
  and the mint spends the row in that same change.
  ✔✔ **THE OWNERSHIP QUESTION IS RULED, 2026-08-17.** Jon, verbatim: *"eventually
  we are going to switch to a Morrowind style inventory, so the occurrence is the
  owner, but inventory likely isn't a count it's a set with a count. I suppose it
  will depend on it the item is unique or not."* — and, correcting my reading:
  *"and when I say set with count I mean dict."*
  ⇒ then, narrowing it twice more: *"when I say set with count I mean dict"* and
  *"ie each item if has a count. for most items it will be a count of 1. note this
  could also be a collection of structs. whatever datastruct makes sense. I'm a
  python guy not a rust guy."*

```text
world       an item is an OCCURRENCE with identity   (held, dropped, placed)
inventory   an ENTRY carrying a COUNT                and the count is usually 1
```

  ⭐⭐ **THE SHAPE IS UNIFORM.** An entry is `(item, count)`; most counts are 1;
  twenty arrows are ONE entry with count 20. There is **not** a unique-item
  representation and a separate stack representation — **uniqueness decides
  whether two entries may MERGE, not how either is stored.**
  ⛔⛔ **"dict" is PYTHON VOCABULARY FOR THE SHAPE, not a mandate for `HashMap`.**
  He said *"whatever datastruct makes sense"* and named himself a Python guy, so
  the Rust representation — map, `Vec` of structs, arena — is the implementer's
  call and choosing one is not overruling him.
  ⚠ **I got this wrong twice before it settled**: first as *"a set of OCCURRENCES
  that reports a count"* (no — 20 arrows are one entry, not 20 identities), then
  as a two-branch unique-vs-stackable model (no — one shape, count usually 1).
  ⭐ each correction made the occurrence model SMALLER, which is the tell that the
  paraphrase was adding structure the ruling did not ask for.
  ⇒ **the rule to design is the CROSSING**: a pickup merges into an entry or adds
  one, a drop MINTS an occurrence, and a unique item's identity survives the round
  trip intact — which is exactly the *"minted instance not in a hand"* case that
  Jon's dropped-weapon ruling already made a prerequisite.
  ▢ **the one genuinely open sub-question**: what makes two entries MERGEABLE —
  an authored uniqueness flag on the definition, or emergent distinguishing state
  (enchanted, named, partly spent). ⛔ do not answer it by inference.
  ⛔ **do not build the general set-with-count today.** *"Eventually"* and
  *"likely"* are his words: the direction is settled, the schedule and the exact
  stacking rule are not. The executable slice stays the gate above, chosen because
  it is the step that is right under either shape — and because it is the one that
  stops a granted row manifesting a second object today.
  `a_granted_quantity_survives_the_death_that_retracts_the_instance_minted_from_it`
  is the poison against retracting the row at the reset instead.
  ⭐ the seam, measured on D125: 5 of 6 catalog classes are counts forever and
  their readers legitimately want a quantity; the whole problem is the **nine held
  weapons/abilities that are an instance and a count at once**. ⛔ the answer is
  NOT a row per object — already rejected — it is deciding which authority owns
  those nine and making the other DERIVE.
  ⚠ related and unclosed: a minted instance **not in a hand** at the commit is
  still undescribed and still lost — exactly where *"a hand needs less than a
  world"* stops paying, because it needs a position.

- ✔ **D134 — CLOSED 2026-08-16. The workspace-policy suite was 12 violations red
  and nothing ran it; it is 34/34 green.** ⭐ **the twelve were four different
  things wearing one label**, and the count was the least informative fact:
  one HAZARD (a `HashMap` the site already sorted before folding — fixed anyway, so
  ordering is the TYPE's property rather than the next editor's discipline), two
  REAL rule violations (`Option<&MotionModel>`, plus a second the rule could not
  spell), two POLICY IMPRECISIONS (a pre-spawn seed and an off-sim scratch: neither
  is an entity, so there is no authority to route through), one contract that had
  OUTLIVED its subject, and seven that were **one wrong fact stated twice**.
  ⭐⭐ **`runtime → ldtk` was never an upward dependency, and `cargo tree` settles
  it in one line** — the ldtk crate's transitive closure contains zero occurrences
  of the runtime or the monolith. Both rules changed, with the argument written into
  their own `rationale` fields. ⚠ `bevy_ecs_ldtk` stays denied in both: the runtime
  may compose the adapter, never the backend the adapter exists to contain —
  poison-tested red to prove that half still has teeth.
  ⭐ **the durable lesson, on its THIRD instance in the same file**: **deleting a
  compatibility facade reddens a boundary policy, every time, and that policy is
  the second file nobody remembers to edit.**
  ⚠ one blind spot left standing deliberately: the movement rule matches
  SPELLINGS, and `Option<&ae::MotionModel>` escapes it. Recorded in that policy's
  rationale rather than widened into a crate this slice did not analyse.

- ✔ **D137 — CLOSED 2026-08-17 (`f7e34225d`). The doc-link ratchet was RED and is
  GREEN — no crate has risen and `--check` exits 0 for the first time since the row
  opened.**
  ⛔⛔ **`--check` IS LOAD-BEARING, measured by poisoning the baseline**: the bare
  command prints *"ROSE from 5"* and **exits 0**; only `--check` exits 1. A step
  wired with the obvious invocation would have been a gate that CANNOT FAIL, which
  is worse than no gate because it reads as coverage.
  ⛔⛔ **and I published a false "it is missing" claim here.** The ratchet had been
  in CI all along (`format-and-clippy`, `ccf254ff2`); I shipped a duplicate step and
  then removed it. **The tell was in my own output** — the grep printed the workflow
  file TWICE and I attributed both lines to the step I had just written. ⇒ **check
  the file as it was BEFORE the edit** (`git show <commit>~1:<path>`). What was
  genuinely missing and is now wired: `cargo test -p ambition_workspace_policy` in
  `engine-tests`.
  ⛔⛔ **a CARVE LAUNDERS DEBT off any ledger keyed by crate name, and the ledger
  congratulates you for it.** The conversation carve took the monolith 122 → 109
  and the script invited banking it — those thirteen were not fixed, they were
  RE-HOMED into a crate the list did not name. Adding `ambition_conversation` put 11
  still-broken links back on the books and moved the honest total 182 → **193**.
  ⇒ the rule is written into `CRATES` beside the entry: **the destination joins in
  the same commit.** ⛔ never run `--update` to clear someone else's rise.
  ⭐ **per-turn gating is answered NO by measurement** — 51 s warm, **338 s after
  touching one crate**, and a gate runs precisely after edits. Pre-push or CI, which
  is where it now runs. ⚠ the cost grows with every carve.
  ⚠ **the residual debt is NOT repaired** (193 broken links): fixing what a session
  broke is not paying it down. ⛔ do not bulk-delete the brackets — that converts a
  detectable break into an undetectable stale sentence. ⭐ the stake, in the
  ratchet's own words: *"a deletion that leaves its references behind turns a doc
  comment into a description of a world that no longer exists — which in this
  repository is where the reasoning lives."*

- ▢ **D166 — THE CHARACTER-AUTHORING BOUNDARY IS CHOSEN BUT NOT YET LOAD-BEARING.
  (from the 2026-08-18 GPT review; the boundary itself is now WRITTEN DOWN)**

`tools/ambition_sprite2d_renderer` is the character-authoring submodule under a
stale name, and as of `7a28709` its README says so in one block at the top:
**it owns character-specific authored MATERIAL and VALUES; this repository owns
the schema, preparation and runtime meaning those values conform to.** The test
for where something belongs is *"is this a VALUE an author chose, or a RULE the
engine enforces?"* ⛔ no rename and no second submodule until the seam is real.

⭐⭐ **AND THE SEAM THE REVIEW ASKS FOR ALREADY EXISTS — measured, not assumed.**
`ambition_characters::prepared` (2,168 lines) is exactly the pipeline the review
draws, and its own header draws it the same way:

```text
CharacterDefinition          authored, decomposable, may reference
      │ prepare_character              validates + flattens
      ▼
PreparedCharacterOverrides   PARTIAL — `None` still means "ask the catalog"
      │ Plugin::finish                 folds the catalog in, ONCE, transactionally
      ▼
PreparedCharacterDefinition  COMPLETE, immutable, no inheritance left
```

⇒ **so the work is not to establish a seam; it is to find what BYPASSES one that
is already built.** Stating it the other way round would have produced a second
half-built pipeline beside a good one.

⭐ **the review's named anti-pattern is ALREADY ABSENT, checked from three
angles** — post-registration reach-in mutation of a character definition:

```text
get_mut::<CharacterDefinition> / .definition_mut / catalog get_mut   0 hits
ResMut<CharacterCatalog | PreparedCharacter* | CharacterDefinition>  0 hits
&mut CharacterDefinition | &mut PreparedCharacterDefinition          0 hits
```

⇒ **games already consume prepared data.** The immutability the review asks for
is enforced by the type, not by convention, so that half of the ask is done.

▢ **what is genuinely open, then:** a fighter's `SmashRepertoire` is authored as
a game-side Rust literal — `george_booul_moveset.rs:556`,
`ambition_demo_sanic/src/smash_moveset.rs:415`,
`pirate_admiral_moveset.rs:461` — rather than as an authored character-package
facet. That is the "scattered game-side Rust constants" the review means, and it
is the one that matters because it is where the next character's values will go.

⚠ **`SmashRepertoire` lives in generic `ambition_characters` and its vocabulary
is not generic** (`ForwardSmash`, `NeutralAir`, posture-sensitive Down-B).
⛔⛔ **do NOT move it for purity.** It is a good abstraction with provisional
ownership; the restitch point is the first real character-owned `smash.fighter`
facet, and moving it before that costs a migration and buys nothing. The
intended direction, recorded rather than built:

```text
Smash capability   defines SmashFighterFacet / SmashRepertoire semantics
        ↑
character package  authors George/Alice/… values
        ↓
Smash preparation  produces runtime MoveSpecs / fighter data
```

⇒ **the generic engine should not need to know Smash move-slot taxonomy** — and
until a facet seam exists, it does, which is the whole content of this row.

⭐⭐ **THE CUSTOMER ARRIVED, AND IT NAMED THE SEAM PRECISELY (2026-08-18, the
grab campaign).** Capture was built to Jon's plan, landed end to end, and then
put enough load on the transitional generic structures to show exactly where the
line is. Five concrete pressures, each measured rather than argued:

```text
BrainSnapshot.captured_for        a capture_* field on the GENERIC snapshot
SpecificAction::CaptureStruggle   a capture verb in the generic action enum
sample_capture_escape             a reader placed specially at BOTH blanking
                                  sites, because no single seam exposes
                                  participant input AND actor-brain output
capture_candidate                 Smash effect KEYS read inside the actor
                                  monolith's option-kit builder
CapturedBy.pummels_landed         platform-fighter state on a generic relation
```

⛔⛔ **and the sharpest evidence is a number the generic scorer CANNOT produce.**
The fighter option scorer prices a move by what it does on CONTACT. A grab does
nothing on contact — its worth is that the opponent is HELD, which depends on
the throw it sets up, the escape risk, the captive's percent and the stage. That
value was briefly modelled as *"the grab is worth its forward throw's damage"*,
and `capture_probe` measured what it bought: the CPU grabbed from **110px with a
42px reach**, nine attempts in sixty seconds, **none of them inside its own
range, zero holds**. The number was reverted to the honest zero.

⇒ so the missing piece is not a weight or a feature — it is that **"how valuable
is holding somebody" is platform-fighter policy living in a scorer shared by
every actor in every game the engine runs.** That is the customer this row was
waiting for, and it is what the capability should own:

```text
generic engine        semantic control transport · body facts · perception
                      · control-hold machinery · temporary relationship and
                      body-constraint primitives · damage / launch
platform fighter      fighter action vocabulary · SmashRepertoire · capture
                      eligibility · pummel / throw / escape rules · what a
                      HOLD is worth to a decision
character package     grab geometry and timing · pummel · throws · weight ·
                      hit/hurt geometry · presentation bindings
```

⚠ **the stopping rule the grab work was held to, worth keeping**: *if a fact
would make no sense in a radically different game that merely has actors and
temporary relationships, it does not belong in the generic actor/character
layer.* The five pressures above each fail it, and each was recorded rather than
deepened once that was clear.
⭐⭐ **AND THE FIRST FACET LANDED — 2026-08-18, George's capture kit as CONTENT.**
Not the sixteen-slot repertoire, and that restraint is the finding rather than a
shortfall: the capture kit is the only part of a fighter's authoring that came
out as pure VALUES — six numbers of geometry, four timings, three payloads, no
helper composition in the middle of them.

```text
ambition_characters::smash_fighter             the facet's SEMANTICS + the schema
ambition_demo_smash/assets/fighters/george.ron the character package's VALUES
CaptureKitAuthoring::into_repertoire           preparation → the SAME MoveSpecs
```

⭐ **the capability is NAMED before it is a crate.** A schema registration must
name an owner, so registering `smash_fighter` writes the ownership down where a
tool can print it — `SMASH_FIGHTER_CAPABILITY`, in the crate the types still live
in. ⛔ it is not a licence to start the carve, and the standing note on both
sibling modules was updated to say so in as many words rather than being left to
read as satisfied.

⭐ **a SECOND pack in the workspace, which is what proves the pipeline is a
CAPABILITY** rather than `ambition_content`'s private loader: the demo compiles
its own pack through `ambition_platformer2d::content`, with no dependency on the
game crate. ⭐ the E9 oracle caught the one leak this needed — parsing `pack.ron`
was `ron::from_str` at every pack owner and `ron` is not a facade re-export — and
it was closed at the FACADE (`ContentPackDraft::from_manifest_ron`), which also
turned Ambition's own manifest panic into a diagnostic.

⛔ **what stayed in Rust, and why it is not a TODO.** The ordinary slots are
authored by COMPOSING `strike` / `impulse` / `on_hit` / `committed_tail` / `feel`,
and George's file states a law about the shape of his whole table in a
`debug_assert` beside them — the one that refused a `0.14` grab for landing in
the gap that IS this character. That composition is the design; flattening it
into RON would trade authored reasoning for a wall of numbers.

⚠ **the ⛔ note this closes.** `smash_capture` used to end with *"if a capture
param ever becomes authorable as loose RON, wiring `ParamSchemaRegistry` is a
precondition of that change"*. They are authorable now and they are not LOOSE:
typed serde with `deny_unknown_fields`, read by the content compiler before a
pack may reach a runtime, so a misspelled `knockback_grouth` names the file and
the field at COMPILE time. The precondition was met by a stronger road than the
one it named.

▢ **still open on this row** — the platform-fighter POLICY half, which is the
part the grab campaign proved the generic scorer cannot answer: *what is holding
somebody WORTH.* The facet moved the VALUES; the decision that spends them is
still `ambition_characters::brain::fighter`'s, and the five pressures above are
still where they were recorded.

⭐⭐ **AND THE OPEN HALF WAS RE-MEASURED, 2026-08-18 — the row's own statement of
it was WRONG.** `capture_probe -- 60` with no `--force`, George versus Alice:

```text
Grab pressed          7 tick(s)   5 of them at 107–122px … and 2 at 11px and 18px
moves started         1 × grab    so SIX presses of seven started NOTHING
attempts requested    3           the one grab's active ticks, all at ~84–93px
holds established     0
inside a grab's 42px  1313 ticks — 36.5% OF THE MATCH
```

⭐⭐ **AND THE SECOND MEASUREMENT NAMED THE CAUSE, because the first one could
not tell "wrong DISTANCE" from "wrong TIME".** `capture_probe` was taught to ask,
at each press, whether the presser was already inside a `MovePlayback` — because
`trigger_moveset_moves` DROPS a requested move outright when one is running and
its cancel window does not permit the new one, which a smash into a grab never
does:

```text
122px  SPENT mid-`smash_down`      11px  SPENT mid-`air_forward`
112px  SPENT mid-`smash_forward`   18px  SPENT mid-`smash_down`
108px  SPENT mid-`smash_down`
107px  SPENT mid-`smash_forward`
115px  the one that STARTED a grab — and 115px is outside its own 42px reach
```

⇒ **ZERO presses from a free body, in 3600 ticks, 1313 of them inside grab
range.** ⚠ instrument caveat, stated because it changes one row: the sample is
taken AFTER `app.update()`, so the tick a grab STARTS reads as "mid-`grab`" — the
115px row. Six of the seven are unambiguous (mid-`smash_*` / mid-`air_forward`,
moves that are not the grab).

⭐⭐ **the causal story, and it is one defect rather than two.**
`REACH_TOLERANCE = 2.0`, so `reach_fit(44, 110) = 1 − 66/88 = 0.25`: the grab
survives the "cannot reach" filter at 2.5× its own reach while every shorter move
is filtered to zero, so **at long range the grab is the last option standing and
wins by default**. And the body is at long range precisely BECAUSE it just threw
a smash and is in its recovery. ⇒ **the grab is chosen exactly when it cannot be
pressed, and never when it could work** — at 11–18px the jab and the smashes beat
it, because they carry damage and it carries none.

⇒ so the missing term is not "grabs are worth their throw's damage" (measured
wrong, above) and not a wider tolerance. It is that a grab's worth in this genre
is **that it beats a shield and leads to a throw** — which is platform-fighter
policy, exactly what this row says the capability should own, now with a match to
point at instead of an argument.

⭐⭐ **AND THE SAME MEASUREMENT FOUND A COMBAT INPUT BUFFER THAT IS DESIGNED,
ROLLBACK-REGISTERED, AND WIRED TO NOTHING.**

⛔ **I first recorded this as "there is no input buffer, and nothing in the tree
said so" and BOTH HALVES WERE WRONG** — corrected within the hour, and the way it
was wrong is the point. I checked `trigger_moveset_moves` and
`ResolvedAttackGesture`, found no latch, and concluded absence without grepping
for a buffer BY NAME. Then `AxisManeuverState`'s own field doc said it outright:

```text
"Buffered MOVEMENT actions (jump/burst/blink press windows). Combat
 buffers (attack/pogo/projectile) stay on the shared BodyActionBuffer."
```

⇒ so the design is recorded, the movement half is REAL (`buffer_jump`,
`buffer_burst`, `buffer_blink`, `coyote_timer`, all inside the rollback-registered
`MotionModel`), and the combat half is this:

```text
BodyActionBuffer { attack, pogo, projectile }   on every actor
rollback                                         registered `body.action_buffer`,
                                                 CANONICAL codec — it costs
                                                 schema and snapshot bytes
production reads/writes of its FIELDS            0
BodyActionBuffer::tick callers                   0
```

⇒ **a press in the last frames of recovery still does nothing**, for a person as
much as for a CPU — the behaviour half of the original claim survives. What
changes is the shape of the work: not "design a mechanic this engine lacks" but
"fill and spend a buffer this engine already declares, already carries on every
body, and already pays to rewind." ⭐ and the rollback question the first note
raised is ALREADY ANSWERED by precedent — `MotionModel` carries the jump buffer
through the same schema.

⚠ still a maintainer's call, because it changes the feel of every character in
every game the engine runs — the same shape as the `REACH_TOLERANCE` question
above, and possibly the same conversation. ⚠ and either way `body.action_buffer`
is currently a row in the rollback schema for state nothing produces: implement
it or retire it, but a canonical-codec component with zero writers is paying
rent.

▢▢ **THE FIX IS A DECISION, AND IT IS NOT MINE TO TAKE UNILATERALLY — three
candidates, costed, 2026-08-18.** All three were reached by asking where "a grab
cannot be thrown from 2.5× its reach" is expressible.

```text
1  per-move tolerance on MoveFrameData   a NEW generic field on the shared
                                         frame-data type, for one genre's
                                         verb ⇒ exactly the pressure the grab
                                         campaign was told not to add
2  ask the REAL question instead of the  REACH_TOLERANCE = 2.0 is a PROXY for
   proxy: tolerance = what the body can  "can I close the gap during startup".
   close during the move's own startup   Deriving it from `startup_s` + whether
                                         the move commands an approach is
                                         engine physics, not genre policy —
                                         and it fixes EVERY move in EVERY game,
                                         which is also its risk
3  the capability ranks its own verbs    what D166's table actually says. ⛔ it
                                         is the carve, and the carve was
                                         explicitly deferred out of product work
```

⚠ **(2) is the one that is principled, and its blast radius is the whole reason
to stop here**: it changes how every CPU in every game this engine runs spaces
itself. ⛔ `ladder_probe` is NOT the instrument for it — it measures self-KO time
against a passive opponent, which is stage awareness, not spacing. The honest
instrument is `capture_probe`'s move histogram plus the fighter option tests,
and a before/after over several seeds.

⇒ **what a reader should do next**: pick one. (1) is cheap and adds a generic
field for one genre. (2) is right and wide. (3) is right and is a carve. Nothing
here is blocked on more measurement — the measurement is done.

✔✔ **AND ONE OF THE REVIEW'S ASKS WAS ALREADY DONE — measured before touching
it.** *"Move authoring has historically duplicated `Vfx(...)` and matching
`Sfx(...)` events … converge on one semantic authored effect request with default
companion sound, while preserving explicit override and explicit silence."*

```text
MoveEvent::Vfx / MoveEvent::Sfx spelled in game/          0 files
                                (the one hit is prepared.rs DERIVING the cue
                                 inventory, which is the seam working)
MoveEventKind::Vfx / ::Sfx authored                       15 / 11
```

⇒ **D149 already made a `Vfx` event carry its companion sound**, so the surviving
`Sfx` events are the explicit standalone/override half the review asks to
preserve, not leftover pairing boilerplate. ⭐ and the doubling risk the
convergence CREATED is guarded by `ambition_content::moveset_sound` — an oracle
built from the two real systems (`dispatch_move_events` + `process_fx_requests`)
rather than a data test, whose one claim is that an authored burst is heard
EXACTLY ONCE. ⛔ nothing to converge here; re-doing it would re-introduce the
doubled jab that guard exists to catch.

- ▢ **D136 — COMPOSITION BOUNDARIES ARE ASSUMED, NOT STATED — so whoever
  installs a thing first decides who pays for it. (PROMOTED from `tracks.md`
  2026-08-16, with five instances measured in one night as its evidence)**

⭐⭐ **AND THE FIRST POSITIVE INSTANCE — 2026-08-17, boundaries that were STATED
did the work, which is this row's thesis run forwards instead of backwards.**

Relocating four modules out of the monolith (`355874fe1`), three "obvious"
destinations REFUSED the work in their own words, before any code moved:

```text
ambition_dialog          declares itself CONTENT-FREE
ambition_settings_menu   renderer-agnostic, carries no bevy
ambition_menu            its manifest says the trimmed bevy features are
                         "load-bearing for the WHOLE workspace"
```

⇒ **every one of those is a composition boundary written down where the next
person looks**, and each turned a plausible move into an obviously wrong one at
zero cost. ⭐ the rule this yields is small and practical: **read the
DESTINATION's stated contract before moving anything into it** — the failure
this row catalogues is discovery-by-collision, and a stated contract is how a
boundary gets discovered by READING instead.

⚠ the counter-case is in the same commit: `items` and `world` could not move
because `construction` imports `world::placements` BACK — a bidirectional edge
nobody declared, found only by chasing it.

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

⭐⭐ **AND THE MONOLITH'S OWN `ldtk_runtime` FEATURE WAS A FICTION — measured
2026-08-18, which is the sharpest instance this row has.**

`bevy_ecs_ldtk` and `bevy_asset_loader` are declared OPTIONAL in
`ambition_platformer2d_actor_monolith`'s manifest and gated behind
`ldtk_runtime`. Exactly one module named both UNCONDITIONALLY, so:

```text
cargo check -p ambition_platformer2d_actor_monolith --no-default-features
  → 4 errors, ALL FOUR in src/assets/loading.rs
```

⇒ **turning the feature off did not yield a smaller crate; it yielded a crate
that would not compile.** The manifest stated a boundary and the code did not
honour it — this row's title with the two halves in one crate instead of two.

⭐ **and the module was reachable only because a dead parameter kept it alive.**
`SimulationSetup` carried `sandbox_data_asset`, `sandbox_asset_collection` and
`asset_server` purely to clone two handles into `_`-prefixed locals that dropped
on the next line. That keeps NOTHING loaded — the resources holding those handles
are what keep the assets alive, and they outlive the call by construction. Five
of the seven call sites already passed `None, None`. Deleting the three params
took the provider's only `AssetServer` dependency and its only mention of the
LDtk asset type with them, and `#[cfg(feature = "ldtk_runtime")] pub mod loading`
then compiled clean.

⚠ **the footprint ratchet did NOT move** (44 linked / 17 unwanted, unchanged) and
saying otherwise would be the easy overclaim here: no Cargo edge changed, because
the optional dep was already declared optional. What changed is that the
declaration is now TRUE. The ratchet measures the sentinel's closure; it cannot
see a feature that is unusable, which is why this instance needed a build to find
rather than a manifest read.

⭐ guarded by a `run_tests.py` job that runs that exact build — the CONDITION, not
a grep proxy for it. ⚠ it sits in the exhaustive plan with the other
feature-variant jobs (a distinct feature set is a distinct dependency graph), so
it catches this on Jon's periodic sweep, not on every backbone run.

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

⇒ **THE INVERSION'S SURFACE, MEASURED 2026-08-18 so the next session starts from
it rather than re-deriving it.** The encounter pair is TWO files and a small
symbol set, not a sprawl:

```text
encounter/loading.rs   217 lines · &LdtkProject, &LdtkLevel, field_string, field_f32
encounter/systems.rs   680 lines · ONE line — Option<Res<ActiveLdtkProject>> at :50,
                                   under the comment that already names W4
```

⭐ **so the cost is not in the consumers — it is in the ROOM IR**, which has to
carry what `loading.rs` reads out of entity fields today (a trigger's `id`, its
`camera_zoom`, an entity's `brain` and `character_id`). ⇒ scope the slice as
*"the IR emits encounter triggers"*, and the two readers follow almost for free.
⚠ do not start it as a file move: relocation is exhausted (above), and every
symbol left is genuinely LDtk-shaped.

✔✔ **AND IT LANDED 2026-08-18 — THE ENCOUNTER PAIR IS OFF LDtk.** `EncounterTrigger`
and `LockWall` were the two markers the converter deliberately DROPPED
(*"read by their own consumers off the raw `LdtkProject`; they never join the
emission stream"*) — that sentence was the dependency. They emit now, and
`load_encounter_specs_from_rooms` reads a `&[RoomSpec]`.

```text
holders in the monolith   5 -> 3   encounter/loading.rs and encounter/systems.rs
                                   no longer name the LDtk crate at all
remaining                          world/mod.rs · world/gated_lock_walls.rs
                                   world/authored_switch_commands.rs
```

✔✔ **AND THOSE THREE ARE ALREADY DONE — re-measured 2026-08-18, and this row was
understating its own completion.** All three "remaining" holders name LDtk in
PROSE only (*"it used to take an `LdtkProject`"*, *"`ActiveLdtkProject::is_changed()`"*
in a comment about what the code no longer does):

```text
monolith PRODUCTION refs to ambition_platformer2d_ldtk    0
the 7 that remain                                          all in `tests.rs` —
                                                           converter/vocabulary
                                                           fixtures, legitimate
monolith PRODUCTION refs to bevy_ecs_ldtk (upstream)       1
```

⇒ **the one surviving production edge is a different KIND of thing**, and worth
naming rather than counting with the others:
`assets/loading.rs` declares `#[asset(path = "game://worlds/sandbox.ldtk")]
pub ldtk_project: Handle<bevy_ecs_ldtk::assets::LdtkProject>` — an ASSET-LOADING
declaration that the app loads the file, not a consumer reading world facts out
of a project instead of the room IR. ⚠ whether an asset collection in the
monolith should name the format at all is a real boundary question and IS this
row's subject; it is simply not the inversion the three-file list implied was
outstanding. ⛔ a reader starting from that list would have redone finished work.

⭐ **the placements channel was NOT the road, and the reason is worth keeping**:
`PlacementSchema` is a CLOSED Tier-0 schema whose `PlacementKind::stable_id` is
*"an explicit compatibility contract [that] may only change with a
fingerprint-schema bump"* — so riding it would have cost a netcode/replay schema
event for a load-time fact. `RoomSpec` feeds no fingerprint (checked), so typed
families are free.

⭐ **three things got MORE correct on the way, each measured before it was
relied on:**

```text
coordinates   the old reader used entity.px RAW (level-local); the IR applies the
              active-area offset. Every encounter area in the shipped worlds is a
              SINGLE level, so that offset is zero and nothing moved — but a
              multi-level area would have been placed wrong before
duplicate ids two levels sharing an area, each with a trigger, produced TWO specs
              under ONE id; a room yields one
readiness     the old code LATCHED `specs_loaded` when the project was absent.
              The room set is a SESSION-ROOT component installed by the same event
              that grants the spawn scope, so absence can now mean "one tick
              early" — latching on it would lose every encounter in the run. It
              returns without latching, and nothing outside that system reads the
              flag (checked)
```

⚠ **the brain vocabulary is the one place the two roads disagreed**, and the
population decided it: the project reader defaulted an ABSENT `brain` to
`"medium_striker"` while the IR defaults it to `Passive`. **Measured: zero
`EnemySpawn` markers exist in any encounter area** — all authored encounters
drive from the wave book — so the fallback has no content and the exact inverse
was taken instead of preserving a default nothing exercises.

✔ **verified live against a pre-change baseline**: the boot census read
`1 encounter entit(ies)` before and reads `1` after. ⚠ and the census line said
*"from LDtk"*, which had become false — corrected in the same commit.
⭐ **poison-verified**: making the trigger converter emit nothing turns all three
loader tests red, and those tests drive the REAL road (shipped world → converter
→ rooms → loader) rather than a hand-built `RoomSpec`, so a converter that stops
emitting fails there instead of at runtime.

✔✔ **AND THE GATED LOCK WALLS FOLLOWED, SAME DAY — 3 → 2.** They read the SAME
`LockWall` marker, so once it emitted, the only thing missing was two fields:
`EncounterLockWallSpec` gained `id` and `gated_by`, and
`authored_gated_lock_walls` takes a `&RoomSpec`.

⭐ **the active-room-id argument went with it, and that is the finding.** The old
signature was `(project, active_room_id)` — it walked levels to find the one it
wanted. **A room IS the filter**; the id parameter existed only because the input
was a whole project. The system already held the room set to name the active
room and then asked LDtk what was in it.

⛔⛔ **AND THE CHANGE SIGNAL HAD TO MOVE WITH THE DATA.** The cache watched
`ActiveLdtkProject::is_changed()` — the hot-reload case its own test was written
for. Watching the room set instead is not optional bookkeeping: a reload that
rebuilds rooms under an UNCHANGED room id would otherwise serve a stale wall set
forever. `swapping_the_room_set_alone_invalidates_the_cached_walls` (renamed `8559d1246`) became
`swapping_the_room_set_alone_…` and now pins exactly that.
⚠ **and the fixture had to grow a `PlayerStart`** — the converter refuses an area
without one, and the old hand-walk never asked. A fixture that could not survive
the real pipeline is a fixture that was testing less than it looked.
⭐ poison-verified: dropping `gated_by` from the emission turns three of the five
red.

✔✔ **AND THE SWITCH COMMANDS FOLLOWED — 2 → 1, and the schema decision was
taken rather than deferred.** `authored_switch_commands` read a `Switch`
marker's `on_activate` off the project. `convert_switch` already emitted the
switch — but through `InteractableSpec`, a variant of the CLOSED Tier-0
`PlacementSchema`, so folding the line in would have put a
fingerprint/replay schema event behind a load-time string that ONE consumer
reads.
⇒ **`SwitchCommandSpec` rides alongside as its own typed family**, the same
answer the encounter and lock-wall roads took, for the same reason. ⭐ the
placement record is untouched, so no schema is owed.
⚠ the change signal moved with the data again (`ActiveLdtkProject::is_changed`
→ the room set), and the third `swapping_the_project_alone_…` test became
`swapping_the_room_set_alone_…`. Poison-verified: dropping the emission turns
four red.

✔✔ **AND THE LAST "HOLDER" IS A DOC COMMENT.** `world/mod.rs:6` says *"W3 moved
it to `ambition_platformer2d_ldtk`"* in a `//!` line. ⇒ **no production module in
the actor monolith names the LDtk crate any more** — 5 → 0 in one sitting,
verified by grep over `src/` with tests and comments excluded.

✔ **THE DEP IS `optional` NOW** — the four forwards name `dep:` and the tests
take it back through `[dev-dependencies]`, which is exactly the pattern the
subsystem-gate comment in that file already describes.

⛔⛔ **AND IT MOVED THE COUNTER BY NOTHING — 44 crates / 17 unwanted before and
after — which is the finding, not a disappointment.** Asked why, `cargo tree -f
"{p} :: {f}"` says the sentinel builds the monolith with
**`ldtk_runtime, portal, portal_ldtk` already on**, so an optional dep is simply
enabled. **Two crates hard-code those features with no gate of their own:**

```text
ambition_sim_view              features = ["ldtk_runtime", "input", "portal"]
ambition_platformer2d_runtime  features = ["headless", "input", "portal_ldtk"]
```

⇒ **that is this row's thesis with a name.** The observation crate and the
runtime each decided that a movement-only game wants LDtk and portals, and wrote
it into a manifest — *"who is this for?"* answered by whoever declared it first,
in the one place nobody reads. ⭐ **and the optional dep is still a precondition,
not a wasted step**: with it unconditional the counter could not move no matter
what those two did.

⛔⛔ **AND THOSE TWO LINES CANNOT SIMPLY BE DELETED — probed, 2026-08-18.**
Dropping `ldtk_runtime`/`portal` from `ambition_sim_view` fails to compile, and
**not in `sim_view`**: the MONOLITH itself does not build without `ldtk_runtime`.
Its own subsystem-gate comment already admits this — *"Code inside these
subsystems is not yet cfg-gated end-to-end, so disabling them today only works
when paired with `--features visible`"* — so the manifest lines are a SYMPTOM of
that, not the cause.

⭐ **and the ungated surface is much smaller than that sentence suggests.**
Measured: **two files**, and one of them is a comment.

```text
assets/loading.rs   30 LINES — one `use bevy_asset_loader::prelude::AssetCollection`
                    and one field `Handle<bevy_ecs_ldtk::assets::LdtkProject>`
session/setup.rs    a doc comment mentioning `bevy_ecs_ldtk`; no code
```

⇒ **the slice is: gate `Platformer2dStartupAssets` behind `ldtk_runtime`**, then
its four consumers, each of which needs its own feature to gate on:

```text
ambition_platformer2d_provider  lifecycle.rs:1181   Res<'w, …>  (NOT optional — the one to look at first)
ambition_platformer2d_actor_monolith  session/setup.rs:20,91   Option<&…>
game/ambition_app  setup_systems.rs:82, plugins.rs:292,327     Option<Res<…>> + `init_collection`
```

⚠ **the counter is the acceptance test**, and nothing before that last consumer
lands will move it — which is why this is written as one slice rather than four.

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

- ✔ **D135 — CLOSED 2026-08-16. The canonical session world carried an
  authoring-format-specific field (`runtime_rooms: LdtkRuntimeIndex`) and five
  RON-authored games filled it with `::default()` for a world they will never
  install.**
  ⭐⭐ **the row's own warning saved the work**: *"do not start this by moving the
  type — ask first whether a RON-only game should carry the field at all."* The
  answer was NO, so the 58-site move-and-rename was never started. Ten readers were
  measured: five are LDtk's own systems, the monolith's was **DEAD** (borrowed,
  silenced with `let _ =`, its promised follow-up never written), and the other two
  duplicate facts already checksummed elsewhere. Exactly ONE site in the workspace
  ever built a non-default index.
  ⇒ the field became `Option<…>`, private, `None` by default, installed only by the
  LDtk road — and **the deletion went further than the field, because the field was
  propping up three other things**: a `demo_fixture` re-export that let RON shells
  hand an empty index to a constructor demanding one (a fixture module laundering a
  format dependency into games that have none); three setup systems, two of them
  RON games *querying the session root for LDtk state in order to boot*; and six
  systems that rebuilt against an empty index every sim tick in five games, now
  `.run_if(ldtk_world_installed)`. **The optional field is what made that statable.**
  ⛔ **two of this row's own numbers were wrong** — the grep swept
  `.claude/worktrees/` clones, inflating 14 `::default()` sites to "~25" and 56
  references to "58". ⇒ **an absence or population count taken with a repo-wide grep
  must exclude worktree clones.**
  ⛔⛔ **`capability-footprint-may-not-grow` DID NOT MOVE, and asking was right**:
  still 42 crates linked, 15 a movement-only game never asked for. ⇒ **the footprint
  is dominated by the MONOLITH, not the session world** — seven production files
  still need nine symbols from the LDtk crate, so the next slice against that ratchet
  is a monolith carve at the world-manifest/asset-catalog seam (D136 carries it).
  ⭐ the guard is two tests that are one claim: *"a RON game has no LDtk index"* is
  trivially satisfied by never inserting the component anywhere — which is what a
  BAD implementation looks like and would delete level streaming while turning the
  file green — so the absence is asserted only beside a positive observation that
  the LDtk-authored game installs a real, **non-empty** index. ⚠ the negative
  fixture was caught not reaching its state and said so rather than passing
  vacuously.
  ⚠ schema stayed at 34, verified rather than assumed: the fingerprint is computed
  from the REGISTRATION list, and no sweep fails on a registered component being
  ABSENT.

- ✔ **DEATH RULES STOPPED BEING A PROCESS-GLOBAL `Resource` (2026-08-16)** — three
  games each inserted one in `Plugin::build` and the last one won, so every Smash
  match in the shipped host ran under Mary-O's death rules.
  ⭐⭐ **the right mechanism already existed, ONE NOUN SHORT.** `mode_scope` scopes a
  hosted game's SYSTEMS and its ENTITIES to the rooms tagged with its mode, and every
  Sanic and Mary-O system already goes through it — it simply had no word for a RULE.
  ⛔ the shell's `ExperienceScopeBuilder` does NOT fit and it is not for want of
  adopters: it releases state on route DEPARTURE and has no entering half, so
  `DeathRules` would be deleted forever on the first departure.
  ⇒ a game declares into `DeclaredDeathRules` under the rooms it governs;
  `governing(mode)` is the one place *"whose rules apply here?"* is answered, reached
  through one `SystemParam` so a third beat cannot re-derive it. A room no game
  claimed reads `LevelReset::Never`, which is exactly what an arena wants; a second
  declaration of one scope panics at build rather than picking a winner. ⚠ the mode
  tag was already universal — the fix needed no new identity.
  ⭐ **what a THIRD instance would have to look like to justify a general
  mechanism**, since this slice deliberately did not build one: not another resource
  (that is now a five-line pattern), but **state belonging to a game while it is NOT
  in its own rooms** — a crossover fighter carrying its home rules onto a foreign
  stage — or a scope whose boundary is a SEAT rather than a room.
  ⚠ the sibling was a smaller cure: `sync_hosted_sanic_wallet_shield` (⚠ GONE — consolidated at `03d4c8d22`) was not a
  global but a system whose POPULATION was every `PrimaryPlayer` in the process. It
  states its population now, and its `hosted` fork is deleted — the constructor flag
  was answering a question the ROOM already answers.

- ☑ **D131 — CLOSED 2026-08-16. Nothing accrued on a clock: four fighters were
  being divided by 1, 1, 60 and 100.** `damage_percent()` is `accumulated / max`,
  and `max` was each character's HOME GAME's authored pool — Mary-O and Sanic author
  `max_health: 1` because they are one-hit-kill platformer protagonists, so one
  point of ordinary damage read as **4200%**. The meter was honest and the division
  was correct.
  ⇒ **the MATCH declares what 100% means** (`MatchRules::pool_over(authored)`,
  applied at both seat sites). ⭐ the tell it was half a decision: a stocks match
  already overruled the character on *whether the pool kills* and left *how big it
  is* with the character. ⛔ DELETED with it: the 2026-07-31 per-character
  workaround that stamped the reference onto the three ids this demo registers —
  right about the symptom, wrong about the owner, and it could never reach the other
  eleven.
  ⛔⛔ **the swap-to-P2 control was right and pointed at the wrong thing.** It proved
  the cause travels with the CHARACTER — and what travels with a character is its
  authored VITALS, not a system. The crossover-plugin hypothesis was FALSIFIED in
  the same run (zero deaths, zero replays). **What crossed the boundary was a
  NUMBER, not a rule.**
  ⚠ **and the cheap confirming experiment would have "confirmed" the false
  hypothesis**: the standalone smash app composes no Mary-O or Sanic provider, so
  its whole cast was stamped with the reference and the bug is structurally
  unreachable there — for a reason unrelated to which plugins are installed.
  ⚠ the second reported defect was the same instrument, not a second bug: the KOs
  are 3 and 24 ticks apart (one instant at 60-frame sampling) and a respawning
  fighter reads 0% **because it respawned**.

- ☑ **D130 — CLOSED 2026-08-16 BY LOOKING. (a) There is no tofu — it is the STAGE
  FLOOR.** The "two lines of ~35 hollow boxes" are the stage's tiles and the "grey
  HUD chips" are blurred rectangles in the parallax backdrop; photographed at 3x,
  each box has a bevelled highlight and a dark border. ⭐ why it read as tofu:
  `--route smash_gameplay` with no roster puts the camera at its default position
  with no subject, so the floor sits alone in an empty frame with no scale cue.
  ⛔ the HUD font fallback is INNOCENT — every string renders correctly in a real
  match, including the `·`. Do not re-open on the strength of the original report.
  **(b) FIXED — `capture_scene` grew the step that carries a POSITION.**
  `--press touch:XxY` sends the pair of real `TouchInput` messages winit emits and
  lets Bevy's own fold system handle them, so the tool drives the PHONE road the
  product ships — generic, not a smash flag. ⭐ **the cause: key taps are EDGES WITH
  NO POSITION**; the tests seat a fighter by moving the cursor and THEN tapping, and
  the tool's bare `Enter` fired wherever the cursor sat. Guarded by
  `the_capture_tools_documented_taps_seat_two_cpus_on_two_fighters`, which drives
  the same literals through the real host.

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

⭐⭐ **A THIRD CRITERION, MEASURED 2026-08-18, AND IT SETTLES CUT-vs-RESTING
WITHOUT A HEURISTIC AT ALL: draw the same painter into a TALLER CANVAS and see
whether ink appears past the boundary.** Denominator-free like the taper test,
but it does not infer — it observes the pixels that were being thrown away.
Cheap, because the painter is a function and the canvas size is its argument.

⛔ **the taper test is right about Mary-O's walk and WRONG about her idle**, so
"is this sheet clipped" is not even a per-sheet question:

```text
small idle    lowest ink 32.00u of 32   0 px past the edge   RESTING — flat soles on the floor
small walk#0  lowest ink 33.00u         78 px past the edge  CUT
grown walk#0  lowest ink 32.83u        115 px past the edge  CUT
```

⇒ flat shoe soles on a bottom-anchored frame arrive at the boundary at full
width and never taper, so idle reads as cut and is not. ⚠ `bottom_center_canvas`
is a plain paste (`y = frame.height - sprite.height`), NOT an ink re-anchor, so
there is no publish step that could put the lost pixels back.

⛔⛔ **AND THE CUT IS THE SYMPTOM OF AN ART DEFECT, NOT A FRAMING ONE — EVERY
WALK FRAME PUTS HER FOOT BELOW HER OWN STANDING LINE, ON BOTH FORMS:**

```text
                idle foot   walk#0    walk#1    walk#2    skid#0
small            32.00u     +1.00     +0.50     +1.00     +0.50
grown            31.67u     +1.17     +0.33     +1.17      —
```

⇒ she walks THROUGH the floor by up to a fifth of a tile, and the clipping is
just the canvas noticing. The small form clips at all only because its idle
already sits exactly on the last row, with zero headroom to dip into.
⭐ **so the fix is the walk pose's leg reach, not a bigger frame** — a taller
canvas would preserve the feet and keep them under the ground. ⚠ pre-existing on
the grown form: its walk frames are byte-identical through the rig refactor.

✔✔ **FIXED 2026-08-18** (renderer `a17b8bf`), and the cause was a SIGN. `+dy` is
DOWN, and the trailing leg carried `leg_back_dy=+1.0` at toe-off: the foot
pushing off was extended THROUGH the standing line rather than lifted off it.
The passing pose's `bob=+0.4` was the same mistake at a third the size — the
middle beat of a walk is its HIGHEST.

```text
            idle foot   walk#0   walk#1   walk#2        after
small        32.33u      +1.00    +0.33    +1.00     →  +0.00
grown        31.67u      +1.33    +0.67    +1.33     →  +0.00
```

⇒ four numbers in two pose tables. Seven frames left the clipping guard's list:
`mary_o_v2` 14→13, `mary_o_v2_tall` 11→8, `mary_o_v2_fire` 6→3. ⭐ the three
CANONICAL images are byte-identical through it, which is what a change confined
to the walk beats should look like.

⭐ **the guard asserts against her OWN idle, not against the frame height** —
her standing line is a property of the form and moves with per-form scale, so
"below the frame" would have been a proxy for "below the floor". Poison-verified.

▢ **what is still cut on her, and it is NOT a pose** — measured 2026-08-18, and
it is THREE numbers that should be one. At 6px per authored unit on a 192px
published frame:

```text
                    drawn sole    authored collision_bottom_px    foot socket
small (one brick)     194 px               190 px                   176 px
grown (two bricks)    190 px               192 px                   176 px
```

⇒ the small form's sole is 2px BELOW its own frame (the sliver that is cut on
every frame of that sheet) and 4px below its collision box; the grown form's is
2px ABOVE its box; and both forms' `foot_r`/`foot_l` sockets are the same
hardcoded `output_px(88.0)`, 14–18px above where either foot actually is. Three
authorings of "where her feet are" that agree with each other nowhere.

⛔ **not fixed here on purpose.** Every repair moves where she STANDS, and that
is the decision D165 records Jon making by eye ("small Mary-O is one brick,
grown is two"). The measurement is the part that was missing; which of the three
numbers is the authority is his call, and the sockets are the one no form
currently derives. `grow`/`shrink` (both sheets) and `death#0` (top)
are separate frames with their own reasons.

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

✔✔ **AND SUPER SANIC — THE SHEET JON ACTUALLY REPORTED — IS FIXED 2026-08-18**
(renderer `39d79a7`). The raised fan and the back blade are scaled by a named
`SUPER_SPIKE_FIT`, swept against the pipeline's own criterion:

```text
x1.00   61 of 181 frames cut          x0.80    2 cut
x0.85    4 of 181 cut                 x0.76    0 cut     <- shipped
```

⛔ **0.76 is the largest value measured that leaves every frame whole** — not a
round number, and the count is 61 rather than this row's 54 because the sheets
moved since, which is the build-time-observation point above making itself.
⭐ **the control is asserted with the fix**: base `sanic` is 0 of 181 on the same
canvas, so a later change that shrank every spike — or grew the frame for
everybody — cannot pass the guard. Poison-verified at 1.0.
⛔⛔ **and "just make the frame bigger" is refused with cause**: `auto_crop` is
OFF for this sheet precisely so `ATTACK_HITBOXES` coordinates match draw space,
so growing the frame or shifting the body silently moves every authored hitbox
with respect to the drawing. ⇒ **for a sheet with authored hitboxes in draw
space, the ART is the only thing that can give** — worth checking before
proposing a canvas change for any of the other ~15 authoring sources.

⚠⚠ **AND THE RENDERER'S OWN SUITE IS RED AND HAS BEEN — 10 failures, measured
2026-08-18 while fixing the above.** It was 11; `test_no_raw_imagedraw` is now
green, and triaging it found a REAL defect on the protagonist (the boot
thruster's nozzle ellipse REPLACED the bloom's alpha instead of compositing, so
the plume lost its glow at the nozzle — 61 of 396 player frames). ⇒ **a guard
that has been red for a while protects nothing, and this one was hiding a
player-visible bug in its own noise.** The rest, untriaged and recorded so they
are visible rather than discovered again:

```text
✔  test_svg_parts_cache      x2   the message named a dependency that was INSTALLED
✔  test_actor_contract       x1   a dead exemption plus a proxy question
✔  test_character_notes      x2   written for a schema that changed under them
✔  test_robot_slash_hitboxes x2   froze one build of the art as the requirement
✔  test_geometry_gui         x1   held a reference the drag handler REPLACES
✔  test_portrait_product     x1   froze which clips a boss draws from
✔  test_rig_codegen_and_scale x1  ⭐ THE ONE REAL ONE — fixed, see below
```

✔✔ **AND THE ONE REAL ONE IS CLOSED — 2026-08-18** (renderer `6162a4a`). A
target generated from a rig document rendered a **different image from the
document it was generated from**: `RigDocument.render_at` downsamples its
supersampled frame through `resize_transparent_sprite(..., reducing_gap=3.0)`,
and its own comment says why — *a Lanczos kernel has negative lobes, so reducing
a silhouette laid over transparency leaves faint non-zero-alpha pixels several
texels outside it*, which is the pale halo. `rigdoc_codegen` emitted
`img.resize(..., LANCZOS)`: exactly the call the interpreter had deliberately
stopped making.

```text
alpha difference across the body   15        tolerance   2
```

⇒ **the halo fix landed on ONE of two roads.** Every character published from a
rig document has been shipping the artifact the interpreter removed. The
generated render now takes the same path, including the `SS == 1` short-circuit.

⚠ **and one of the seven above was mis-triaged by ME before being fixed**:
`test_oiler_svg_rig` raised rather than skipped when `resvg_py` is absent, unlike
its three sibling SVG suites which all open with `pytest.importorskip`. ⛔ that
is NOT the trap this row records two paragraphs down — there the package was
INSTALLED and the message was two layers from the cause. Here `pip show` finds
neither `resvg_py` nor `cairosvg`, so the skip is honest, and in an environment
that HAS the wheel the guard passes and the tests run exactly as before. **Check
which world you are in before adding a skip; the check is one command.**

⇒ the renderer suite is now **1 failed / 559 passed**, and the last one is the
Mary-O visual baseline standing against an uncommitted side-view strap edit in
the worktree. ⛔ **that hash cannot be re-recorded on its own**: recording it
without the art that moved it leaves the committed baseline disagreeing with the
committed art. It belongs to whoever lands the art, in the same commit.

⛔ **do not bulk-fix these**; each is a different question, and two of them are
about whether the assertion or the art is right — which is a look-at-it call.

⭐⭐ **AND THE ONE THAT LOOKED LIKE THE CHEAPEST WAS THE MOST MISLEADING, which
is why the list above quotes what each failure SAYS rather than what it means.**
The two SVG-cache failures read *"SVG sprite rendering requires native
resvg-py"*, so the obvious repair was to skip them when the wheel is missing.
**`resvg_py` 0.3.3 was installed the whole time.** `_native_resvg_callable`
requires `inspect.isbuiltin` — *"never a Python compatibility shim"* — so the
tests' pure-Python fake was refused BY DESIGN, the rasterizer fell through to
CairoSVG, and CairoSVG's absence raised a message about resvg. Two layers from
the cause.
⇒ ⛔⛔ **skipping them would have "fixed" a dependency that was present, and left
the real gap in place**: the isbuiltin rule was asserted NOWHERE, which is how
two tests could lean on it, break when it tightened, and read as an environment
problem. It has its own poison-verified test now, and the suite is **8 failed /
619 passed** (renderer `2b9d160`).

⭐⭐ **AND THE NEXT ONE DOWN WAS THE SAME DISEASE.**
`test_every_registered_character_target_has_local_actor_metadata` was red on the
Perfect Cellular Automaton, and the content was fine both times:

```text
its EXEMPTION was dead      it skipped `rigged.TARGETS` because rig-doc targets
                            "do not yet carry actor metadata" — that set is
                            EMPTY, and every rig-doc character but one carries
                            metadata perfectly well
its QUESTION was a proxy    "has a module-level ACTOR_METADATA constant", where
                            PCA authors its metadata in its `.rig.json` and
                            hands it to `build_sheet` at render time
```

⇒ the guard asks whether a character publishes metadata by SOME road now, and
the target exposes `actor_metadata()` — a function, not a constant, because the
constant would parse the rig document at IMPORT time for every discovery.
⛔ **and the helper's first draft guessed the attribute name** (`module_name`,
`module`; the record carries `module_path`), returned `{}`, and left the test red
for a NEW reason that looked exactly like the old one. **A guessed attribute
fails as silence.** Poison-verified. Suite **7 failed / 620 passed** (renderer
`24b10cb`).

✔✔ **ALL OF THEM TRIAGED 2026-08-18 — 11 → 1, and TEN WERE BAD TESTS.** Every
repair is in the renderer's history with its evidence; the pattern is the row's
real product: **on this list, read what the check computes before believing what
it reports.** Two named a dependency that was installed, two froze one build of
the art, one froze a boss's clip list, two were written for a schema that changed
under them, and one held a reference to a dict the code deliberately REPLACES —
that last read as *"dragging a polygon moves it by (0,0)"*, which is the
expensive kind of red, because it accuses the code.

▢ **AND THE SURVIVOR IS GENUINE, LEFT RED ON PURPOSE.**
`test_generated_matches_rigdoc_render` compares the rig document's own renderer
with the module generated from it, and they disagree on every clip:

```text
alpha delta       max 11, >2 on 355 px, >8 on 8 px   (the tolerance is 2)
visible delta     max 19.8/255 on 137 px, mean 0.32
alpha bbox        rigdoc 39x78 vs codegen 43x82 — a 2px fringe all round
NOT a translation every 1px shift makes the match WORSE
```

⇒ composited, the two pictures are indistinguishable; the disagreement is a
sub-pixel rasterization difference. ⛔ **do not "fix" it by re-basing the
assertion on a visible-difference metric** — that turns the light green and
hides the fact that two roads which are supposed to emit one picture no longer
do. What is unknown is what introduced it.

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
CHECK that refuses to publish a clipped frame, and (b) the 23 sheets that are
already clipped.
✔✔ **(b) IS RULED 2026-08-17: CASE BY CASE, DRIVEN BY THE WARNING.** A sheet is
fixed when its clipping is actually VISIBLE in play, and whether it is fixed by
growing the logical frame or by re-authoring the art is a per-sheet call.
⛔ **do not open a 23-sheet campaign and do not bulk-grow frames.** ⇒ what is left
of this row is (a) plus the standing guard, and the population stays known-bad by
design — which is exactly why the ordering rule below is load-bearing.

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

- ◐ **D128 — Can this engine carry a serious platform fighter through ORDINARY authoring? (product-pressure vertical slice, opened 2026-08-15; ⭐⭐ EVERY ENGINEERING LINE IS ✔ AS OF 2026-08-18 — what is left is Jon playing one match)**

⭐⭐ **READ THIS BEFORE ANYTHING ELSE IN THE ROW: the executable list below is
EMPTY.** Pacing was ruled, respawn placement and asset composition landed, CPU
symmetry landed, and all four presentation defects are closed — 5 and 6 fixed and
photographed, 7 was fixed three hours after it was reported, 8 verified live.
⛔ **so do not open this row looking for work**; it stays ◐ only because its
QUESTION is a product one and the answer is a person watching a match, not
another capture.

⭐⭐⭐ **ACTIVE TRUTH — 2026-08-17, and it is the ONLY executable statement in this
row. ⛔⛔ EVERYTHING BELOW IS EVIDENCE IN REVERSE-CHRONOLOGICAL LAYERS, AND EVERY
`⇒ NEXT` / `⇒ the cheap next step` / `⛔ the next spend` sentence in it is
SUPERSEDED BY THIS BLOCK.** This row accumulated seven separate "next" claims
across three days; several of them were carried out and the instruction was left
standing, which is how a compacted agent re-does landed work.

✔✔ **PACING IS ACCEPTED — RULED 2026-08-17, so this row no longer waits on
anybody.** Jon, verbatim: *"A three-stock CPU showcase finishing in under ~40
seconds is certainly not too long; if anything it is brisk. **Do not retune stock
count, knockback, or damage** around that partial 20-second frame. … Human-vs-human
balance can be judged separately later."*

⭐⭐ **AND HE CORRECTED HOW THIS ROW READ ITS OWN CAPTURE, which is the reusable
lesson.** The row led with the 1200-tick frame — 180%/124%, nobody dead — and
buried the whole-match fact two paragraphs down: **at 2400 ticks (~40s) the match
had COMPLETED and returned to CHOOSE YOUR FIGHTER.** ⇒ **when a capture sweeps
time, the acceptance question is answered by the LAST frame, not the most alarming
one.** A partial observation was framed as the finding while the complete one sat
underneath it.

⇒ **so what is left in this row is ENGINEERING, not acceptance:**

```text
✔  seat-independent respawn placement           FIXED 2026-08-18 (defect 3)
✔  standalone smash-app asset composition       FIXED 2026-08-18
✔  the residual presentation defects            5 FIXED + photographed · 6 FIXED (it DID
                                                reproduce; the 08-18 "not reproduced"
                                                was a 40x40 scan window hunting a 19x19
                                                artifact) · 7 was already fixed 08-16,
                                                3h after it was reported · 8 FIXED + live
✔  same-character CPU symmetry — FIXED 2026-08-17, see below
⛔ NOT on this list: stock count, knockback, damage. Ruled. Do not retune them.
⛔ NOT on this list: another ladder run.
```

⚠ **"another capture is done" was true of the ACCEPTANCE question and NOT of the
defects.** Captures on 2026-08-18 confirmed defect 5 live, verified defect 8's
fix end to end, and — once the warmup landed on **360**, the tick the report
actually names — caught defect 6 in the act. None of it was settleable by
reading the code, and the two warmups sampled first (300, 420) settled nothing. ⭐ **and the documented tap recipe still
works**, which the row flagged as the thing most likely to rot: the nine taps
seat two CPUs and start a match unchanged.

✔✔ **CPU SYMMETRY: TWO CPUs WEARING ONE CHARACTER WERE THE SAME MIND, AND THAT IS
FIXED (2026-08-17). Emmy Ethereal now AUTHORS the old behaviour as her own
trait.**

⛔⛔ **the defect, and it was stated as a goal in its own comment.** The fighter
brain seeded its noise stream from difficulty alone —
`0x5F37_7A11 * (level + 1)` — so any two CPUs on one rung drew byte-identical
noise. Reading a symmetric stage, they mirrored each other exactly, and a
same-character CPU-vs-CPU match was one fighter played twice. ⭐ **the tell was
local**: every OTHER template in `brain_builders.rs` (Smash, brute, skirmisher,
sniper, aerial) already varied off `seed_from_id(&enemy.id)`; the fighter was the
sole outlier, so this was the file's own rule reaching the one brain that missed
it.

```text
ordinary   seed_from_id("<character>#seat<n>") ⊕ level    distinct per PARTICIPANT
authored   seed_from_id("<character>")         ⊕ level    shared by every twin
```

⭐ **`enemy.id` was already the right input** — `PreparedSeat::feature_id` mints
`"<character>#seat<n>"` precisely so a mirror match is two bodies rather than one.
⛔ no clock, no process-global RNG, no Bevy `Entity`: **replay determinism is
preserved**, and `the_same_participant_rebuilds_on_the_same_stream` pins it.

⛔⛔ **AND THE FIX NEEDED TWO SITES, WHICH IS THE PART THAT WOULD HAVE BITTEN.**
`project_authored_fighter_ladder` rebuilt `FighterState` with the same level-only
constant on `Added<Brain>`, so fixing construction alone would have been undone a
moment later by the second writer. It now CARRIES `state.noise` across the
rebuild — which is also the more honest operation, since a fighter's position in
its own stream is not one of the profile-cached fields that pass exists to
re-derive.

⭐⭐ **EMMY'S EXCEPTION IS AUTHORED, NOT INHERITED FROM THE BUG.**
`CharacterDefinition::preserves_mirror_symmetry` (authored in
`authored/npc_emmy_noether.rs` as `.preserving_mirror_symmetry()`) drops the
PARTICIPANT term and keys on the character instead, so her twins share a stream
and nobody else's do. ⛔ it does **not** zero the seed — that would hand every
mirror-preserving character one global stream — and it touches nothing but the
choice of stream: no profile, no difficulty, no template.
⛔⛔ **it synchronises NOTHING per tick.** The mirror is *identical cognition +
symmetric information → symmetric behaviour*, so it BREAKS when their
observations diverge, and that is correct — `the_same_seed_shown_a_different_world_may_decide_differently`
is the falsifier for anyone who later tries to enforce it.
⚠ **no `if character == Emmy` anywhere in the AI**: generic code reads a bool that
the character authored, and the trait rides `ActorConfig` (a
`rollback_component_clone`) so seating, room spawn and rewind rebuild all agree.

⭐⭐ **MEASURED ON THE REAL STAGE, and the measurement is worth more than the fix
statement.** `two_cpus_wearing_one_character_stop_being_a_perfect_reflection`
drives a whole same-character two-CPU match through the shipped shell. The stage
seats the pair at **x=224 and x=416 about a midline of 320** — genuinely mirrored
spawns — and under the old shared stream the two bodies stayed an **EXACT mirror
image for the entire match**: equal and opposite about the midline, identical y,
to the float. ⇒ *"perfectly symmetric"* was literal, not impressionistic.

⛔⛔ **AND THE FIRST DRAFT OF THAT TEST WAS VACUOUS, which is the reusable
lesson.** It measured the DISTANCE between the two bodies and passed with the
defect fully present — two fighters spawned apart drift regardless of what their
brains do, so it was measuring collision. The metric that answers the question is
**MIRROR ERROR**, `|(x0−mid)+(x1−mid)| + |y0−y1|`, which is ~0 for a reflection
and grows for two fighters. ⇒ *when the report says "symmetric", measure symmetry,
not difference.* Both halves are poison-checked: the fix, and the ladder
projection's carry, each independently turn tests red when reverted.

✔✔ **AND EMMY IS NOW PINNED IN THE FULL HOST TOO** —
`game/ambition_app/tests/smash_cpu_cognition.rs` (ungated, so the project gate runs
it). ⛔ **the standalone smash app cannot seat her**: it does not compose
`ambition_content`, so a roster naming `npc_emmy_noether` seats nobody, and every other
suite either registers a synthetic stand-in or tests a different link. ⇒ the claim
*"the character a player can actually pick off the grid gets the shared stream"* was
asserted nowhere until this file. It drives `build_visible_app`, asserts
`npc_emmy_noether` is on the assembled grid (`SmashRoster::assemble`), seats two CPU
Emmys through the stage's own roster builder, and reads the streams off the seated
brains. ⛔ do not teach the demo host Ambition's cast to close the older gap.

⭐⭐ **MEASURED IN THE FULL HOST, rung 5, two CPUs on one character:**

```text
                     streams     mirrored for       match ran
npc_emmy_noether     IDENTICAL   2576 of 2576 fr    2576 fr   a stalemate — they
                                                             answer every move
                                                             with its reflection
npc_pirate_admiral   DIFFERENT    488 of 1548 fr    1548 fr   they fight, and it
                                                             ends
```

⭐ **the shorter ordinary match is itself the finding**: fighters that think
differently resolve their match, while two Emmys mirror each other into a much
longer one.

⚠⚠ **488 frames is ~8.1s, and Jon reported it from play**: *"it took a while for
Booule to desync, but they eventually did. And Emmy never desynced. Still the
desync for non-Emmy CPUs probably should happen sooner."*

⛔⛔ **THE CAUSE IS NOT THE SEED, AND TWO FIXES WERE BUILT, MEASURED AND REVERTED.**
The stream has exactly ONE consumer in the fighter brain — press-timing jitter, only
on a decision that commits to an attack — so **a different RNG cannot separate two
bodies doing the same thing**, and both fighters open the match walking toward each
other. They do diverge from frame one (0.0002px against Emmy's 0.00003px of float
noise) but sub-pixel until it compounds.

```text
per-participant DECISION PHASE   488 → 220 fr, and BROKE FIVE behavioural guards
                                 in `the_stage_kills`: a 0-4 tick offset changed
                                 whether attacks connect — "the brain travels but
                                 never commits". Reverted: too high a price.
cadence DRAWS from the stream    220 → 219 fr. Nothing. A staggered decision is
                                 not a DIFFERENT decision. Reverted.
```

⇒ ▢ **what would actually move it is asymmetric CIRCUMSTANCES, and it is already on
this row's list: per-seat spawn placement (defect 3 above).** Two fighters who start
somewhere different take a genuinely different first decision. ⚠ it will also
shorten Emmy's mirror, for a good reason — the assertion that would notice says so.
⛔ do not build a third randomness fix before reading the note at
`brain_builders::fighter_cognition_seed`.

⚠ **the catalog PREVIEW brain still seeds level-only, correctly** — a preview has
no match and therefore no participant; the note at
`character_catalog/resolver.rs` says so and forbids copying it back onto a
construction road.

⭐ **the state acceptance was given against** — two-CPU match captured 2026-08-17
through the shipped shell, AFTER D155 gave the game working knockback: 34%→180% in
thirteen seconds, real exchanges with hit VFX, the stock loop closing on its own.
⇒ **every feel judgement recorded anywhere in this row before that date was made
on a build where nobody was ever launched, and is void** — the same reasoning Jon
used to supersede D114's prohibition.

⛔ **the six named defects from the 2026-08-16 photo session — do not re-derive
their status, and do not re-run the capture or the ladder rig to get it:**

```text
1 self-KO on every stock  ◐ substantially repaired — the CAUSE was architecture
                            (RecoveryLens, 2026-08-15), and the 2026-08-17
                            ladder re-run reads d0 no self-KO / d12 first at
                            21.8s. ⛔ the RecoveryPolicy ledge-grab diagnosis is
                            RETRACTED and banned; a photograph falsified half
                            the CPU-suicide finding.
2 camera loses the fighter ✔ CLOSED — `frame_the_cast` always framed every live
                            seat; three downstream sites threw it away (room
                            clamp, stable_center, 8 Hz ease). Guarded by
                            `every_live_fighter_stays_inside_the_frame`.
3 both seats respawn at    ✔ CLOSED 2026-08-18 — `respawn_placement` takes the
  ONE overlapping point       SEAT. Seats alternate outward from the centre
                              (0 left, 1 right, 2 further left…), so the
                              arrangement is symmetric at any roster size and no
                              seat is privileged; a growing offset would push
                              seat 3 twice as far out as seat 1 for no reason a
                              player could read. Guarded by
                              `every_seat_comes_back_to_its_own_point_on_the_platform`,
                              which pins three properties — no two within a body
                              width, symmetric about the centre, and every seat
                              still ON the platform (an unbounded offset would
                              respawn seat 7 into the blast zone, a worse bug
                              than the overlap). ⚠ and the existing
                              `a_respawn_is_above_the_stage_centre` asserted
                              `respawn.x == centre.x` — the defect stated as an
                              invariant — so that clause was corrected rather
                              than deleted; the height is that test's subject.
4 winner card names a SEAT ✔ CLOSED by D140/D148 — the card reads
                            `WINNER: Robot v3`, and a team keeps its team name.
5 barks draw as a          ✔ **CLOSED 2026-08-18, photographed both ways.**
  screen-wide caption         The OVERLAP half closed via D158→D159 (a bubble is
                              a `WorldLabel` in the one ranked placement pass).
                              The SCALE half was still live and is now fixed:

```text
before   the bark block spans 535px of a 1280px frame, straight across the
         play area, both fighters underneath it — one 265-WORLD-UNIT line on a
         640-wide stage, 41% of it
after    185px, wrapped into a centred column over the speakers  (−65%)
```

                              ⛔⛔ **the cause was that a bark had NO WIDTH AT
                              ALL** — `spawn_speech_bubble` set `font_size` and
                              nothing else, so a line laid out however long its
                              words made it. D158→D159 stopped bubbles
                              overlapping EACH OTHER; nothing stopped one
                              overlapping the GAME.
                              ⚠ **width only, never height**: `TextBounds`' own
                              doc says characters outside the bounds after
                              wrapping are TRUNCATED, so a height bound would
                              silently eat the end of a long bark — worse than a
                              wide one. ⭐ and the four outline children take the
                              SAME bound, or the shadow is four ghosts at four
                              offsets.
6 untextured olive quad    ✔ **CLOSED 2026-08-18 — IT REPRODUCES, IT IS NOT
                              WHAT THIS ROW SUSPECTED, AND THE FIX IS THAT ART
                              THE ENGINE ALREADY SHIPS NOW GETS ASKED FOR.**
                              Photographed at **warmup 360**, the exact tick the
                              f360 observation names — a hard-edged untextured
                              quad, `srgba(1.0, 1.0, 0.35, 0.82)`, spanning
                              x 549..567 / y 392..410 with a ONE-PIXEL cliff on
                              all four sides.

```text
                            tile pitch 25.5px over a 16-unit grid  =>  zoom 1.594x
                            quad          19px  ->  11.9 units   spawn_impact splats 12.0
                            largest VFX  107px  ->  67.1 units   FX_DEFAULT 56 x scale 1.20
                            fighter body  73px  ->  45.8 units
```

                              ⛔⛔ **the cause is `spawn_impact`, and it is not a
                              fallback firing** — `note_effect_miss` logged
                              NOTHING for the whole match, so no authored effect
                              failed to resolve. `VfxMessage::Impact` is the
                              most-drawn effect in the game (every actor hit,
                              projectile hit, pickup and grapple writes one) and
                              it drew a bare rectangle by design.
                              ⭐⭐ **the art was on disk the whole time**:
                              `generic_action_fx` ships `hit_soft`, `hit_hard`,
                              `hit_metal`, `hit_energy`. The marker was a
                              consumer that never joined — the same shape as the
                              189-rows-on-disk / 5-reachable finding
                              `ambition_sprite_sheet::fx` was built to close.
                              ⇒ `spawn_hit_marker` draws `hit_soft` at 0.9 x
                              `FX_DEFAULT_WORLD_SIZE`; `spawn_impact`'s quad
                              survives ONLY as the no-decoded-sheets fallback,
                              so a headless composition looks exactly as before.
                              ⚠ **and the row's own suspect was wrong**: the
                              feature-colour fallback in `rendering/actors/mod.rs`
                              has no olive in its table (hazard RED, actor
                              BLUE/red, breakable BROWN, chest AMBER, pickup
                              MINT, switch RED) and never draws effects at all.

                              ⛔⛔ **AND 2026-08-18's FIRST VERDICT — "NOT
                              REPRODUCED, three frames, both rosters" — WAS AN
                              INSTRUMENT ARTIFACT, published in `bd11b73a4`.**
                              The scan flagged any **40x40** window that was >95%
                              one colour. The artifact is **19x19**. It could not
                              have filled one window at any position, so the
                              measurement could only ever return nothing.
                              ⇒ *a systematic scan is only as fine as its window,
                              and "I scanned everything" says nothing about the
                              things smaller than the probe.* The frames were
                              right; three of them contained it; the sieve had
                              holes bigger than the stone.
                              ⚠ two warmups were also simply wrong: 300 and 420
                              were sampled and **360, the tick the report names,
                              was not.**
                              ⇒ FOLLOW-UP, deliberately not taken here:
                              `ImpactMaterial` (flesh / robot / metal) and the
                              sheet's four hit rows are two vocabularies that
                              already exist and are still unjoined. The material
                              lives on the victim's `HurtFeedback` and
                              `VfxMessage::Impact` carries only a position, so
                              joining them is a message change plus a taste call
                              — Jon's, not mine. Asked as **§18** in
                              [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md),
                              where the taste half is stated: `hit_hard` is a
                              STRENGTH distinction, not a material one, so
                              "material picks the row" explains only three of the
                              four rows the sheet ships.
7 VFX authored against no  ✔ **CLOSED 2026-08-18 — ALREADY FIXED ON
  size reference              2026-08-16, THREE HOURS AFTER IT WAS REPORTED.**
                              The f360 observation was committed at 08:09
                              (`39dc7a39b`); `d6d5810b8` landed at 11:31 and its
                              message names this exact defect: *"`let render_size
                              = BVec2::splat(132.0 * scale)` — inline, and every
                              effect in the project."* It is
                              `FX_DEFAULT_WORLD_SIZE = 56.0` now, plus a per-move
                              `Vfx { scale }`.
                              ⭐ **and the arithmetic closes against a live
                              frame.** At the measured 1.594x zoom the old 132
                              units drew at **210px** (reported "~250"); the
                              largest VFX in the new warmup-360 frame is
                              **107px** — 56 units at the Admiral's authored
                              scale 1.20 — around a 46-unit fighter. It no longer
                              occludes the fighter or the stage.
                              ⛔ **so this row sat ▢ OPEN for two days over
                              landed work**, and 2026-08-18's "NOT reproduced
                              either" was the fix working, read as an absence.
                              ⇒ *before photographing a defect, `git log` the
                              file it names — an observation older than the
                              commit that fixed it is not evidence about today.*
8 capture_scene prints no  ✔ CLOSED 2026-08-18, VERIFIED LIVE on a real
  pose for a 2-CPU match      two-CPU match: `seat 0 at (350.4803, 276.0000)` /
                              `seat 1 at (233.9185, 276.0000)`, where it printed
                              NOTHING before. It reports the SEATED
                              bodies when nobody is driving, one line per seat,
                              sorted by SEAT rather than by query order (Bevy
                              iterates by archetype, so an unsorted list would
                              make two captures of one match differ because the
                              rows moved). ⛔ and when there is neither a primary
                              player nor a seated body it says `NO SUBJECT …
                              this image proves nothing about a pose` — the old
                              `if let Some` printed nothing, so "no pose line"
                              and "no subject" were indistinguishable in a tool
                              whose whole job is to stop a verification
                              photographing the wrong thing quietly.
```

✔ **CLOSED 2026-08-18 — the standalone `game/ambition_demo_smash_app` binary
composed no asset install at all** (no `PlatformerAssetsPlugin`), so nothing
sheet-driven had art in that process. ⚠ Smash reached through the shell is a
DIFFERENT composition and was always fine — say which binary any claim is about.

⭐⭐ **it was the THIRD demo shell and the only one that never joined.**
`ambition_demo_mary_o_app` and `ambition_demo_sanic_app` both install the asset
umbrella and the generic presentation after their composition registers
catalogs, and their twin comments call each other *"the regression test for the
helper an external consumer now depends on"*. ⇒ the fix is the reference shape,
not a new idea: `PlatformerAssetsPlugin::for_experience(SMASH_EXPERIENCE)`
`.with_room(smash_stage().metadata)` then `PlatformerPresentationPlugin`, AFTER
`compose_smash_shell` because the plugin READS the catalogs it registers.

⚠ **`visible` only, deliberately** — `build_demo_app` is also this crate's test
harness, and its 33 regression tests assert on a stepping simulation rather than
on pixels. Sanic draws the same line by keeping its asset install in
`build_windowed_demo_app`.

⚠⚠ **WHAT IS AND IS NOT VERIFIED, because the difference matters here.**
✔ 33 headless tests still pass and both feature configurations compile.
⛔ **the plugin's `build()` was NOT executed** — the crate's tests are gated out
under `visible`, so `cargo test --features visible` links and runs ZERO of them.
⇒ the ordering rests on matching two working shells and on the plugin's own
panic, which names the composition-order mistake rather than booting art-less.
▢ **a windowed run is what closes the loop**, and it is the one thing this
session could not do.

⛔ **what is answered and must not be re-asked**: *"do the two authored kits read
as different fighters?"* — YES, measured inside four seconds. *"is the VFX/SFX
road reachable?"* — YES (`ebc8877ee`, an effect is a name). *"has anybody watched
a match?"* — YES, twice.

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

⇒ **so the capture is a mid-match MOMENT, not evidence of a broken KO** — ⛔ do not
re-open it as a defect on the strength of a screenshot showing a high percent.
✔ **and the PACING question this paragraph raised is ANSWERED: Jon accepted it
2026-08-17** (*"under ~40 seconds … if anything it is brisk"*). ⛔ do not retune
stock count, knockback or damage.

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
⚠ **at 1200 ticks NEITHER fighter had lost a stock at 180%/124%.** ✔ **shown to
Jon and ACCEPTED 2026-08-17** — and ⭐ **he read past this line to the one above
it**: the match had COMPLETED by 2400 ticks (~40s), *"certainly not too long; if
anything it is brisk"*. ⇒ **the 1200-tick frame was a partial observation
presented as the finding**, which is why it read as a pacing problem. ⛔ do not
retune stock count, knockback or damage around it.

✔ **AND THE CAPTURE FOUND ONE REAL PRESENTATION DEFECT — CLOSED as D158→D159 the
same day, so ⛔ do not work it: speech bubbles STACKED ILLEGIBLY.** (The fix was
to make a bubble a `WorldLabel` in the one ranked placement pass, not to retune
the offsets named below.) The 1200-tick frame has three lines drawn over one
another —
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

◻ **HISTORY — this said *"the cheap next step for this row is one capture of a
two-CPU match"*. IT WAS DONE, TWICE (2026-08-16 and again 2026-08-17 after
knockback worked). ⛔ do not run it a third time to establish status; read the
ACTIVE TRUTH block at the top of this row.** The invocation is still the nine-tap
command in `capture_scene`'s header — ⚠ with the corrected cell literals, because
the documented taps once seated the wrong pair.

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

◻ **HISTORY (2026-08-16). Its verdict — *"not yet"* — was OVERTAKEN by D155's
knockback fix and then by Jon's acceptance of the pacing; ⛔ its closing
instruction is spent.** What survives is the half that was never in doubt: with
George Booul and the Pirate Admiral seated, a watcher sees two mechanically
different bodies inside four seconds — different silhouettes, different effects, a
cutlass against a Boolean ghost, 36% traded — so **"content underuse" is answered:
the authored kits DO read**. ⚠ the rest of the paragraph described a build where
the match ended in 13-23 seconds with every stock lost to the void at nearly zero
percent, the camera on an empty platform, and the card announcing *"seat 2 wins"*:
**the camera and the winner card are both fixed, the launches were inverted at the
time, and the current match runs ~40s with real exchanges.** ⛔ **its parting
instruction — *"the next spend is the stage-return loop (1 + 2)"* — is spent**: 2
is closed and 1's cause was architecture, already repaired.

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
