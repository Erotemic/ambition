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

⛔ Refill this table when a lane returns; never leave it describing the last run.

⭐ **REFILLED 2026-08-17.** D125's lane CLOSED (the start-room seam landed and all
24 callers migrated). D33 ran three slices and is parked with the monolith
**under** its frozen baseline.

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

⭐⭐ **THIRTEEN ENTRIES IN THAT FILE WERE RULED ON 2026-08-17** — re-read it before
promoting from this table. ⛔ **two of my own rulings that day were WRONG and are
marked as withdrawn in place**: couch play is NOT switched off, and a clipped
label is not a defect.

| Observation | Why it is worth a lane |
|---|---|
| Super Sanic's spikes are clipped by the sprite renderer | ⭐ Jon called it structural himself — *"we should not be able to clip sprite artwork so easily"*. This is the only one that is an ENGINE gap rather than content |
| ~~Mary-O secret/invisible blocks keep their brick texture when spent (quasar brick in 1-1)~~ | ✔ **FIXED 2026-08-19** — the new arm is guarded on `is_spent`, so an unpaid brick still names no art and still keeps the level's paint. Guarded by the whole look × spent table |
| ~~Mary-O allows one fireball; should allow two~~ | ✔ **ALREADY DONE**, ruled 2026-08-17 — `MAX_LIVE_SPARKS` is 2, guarded by counting LIVE SHOTS rather than reading the constant back |
| ~~the multi-coin block's coin-pop VFX~~ | ✔ **RESOLVED 2026-08-15** — landed in `943a9aa0c`; four demo shells had no `VfxMessage` reader, so it drew in the full game and nowhere else. ⛔ the doc entry said otherwise for a day |
| the snake and AI slop are far too big, and the snake sprite may not match its box | ⚠ related to the player-side sprite/box unit mismatch at the top of that file — the two may be one bug |
| **Sanic is very small in his own game** (Jon, 2026-08-15) | ⭐⭐ **third body in the sprite/box cluster, and the one that makes it a CLUSTER rather than three bugs** — see the measurement below |
| ~~drop `pocket` and `versus` from the main game-selection shell~~ | ✔ **DONE**, confirmed by Jon 2026-08-17. Both call `.unlisted()` (`demo_pocket/src/lib.rs:190`, `app/versus.rs:905`) and `launch_entries()` filters them |

### ✔ Found in passing 2026-08-19 — THE TWO DEMO APP CRATES ARE OUTSIDE THE PER-TURN GATE AND CI

⛔⛔ **CORRECTED 2026-08-20**: the per-turn gate (`cargo test --workspace --lib`,
`cargo check -p ambition_app --all-targets`) and CI (`.github/workflows/test.yml`,
which names `ambition_workspace_policy`, `ambition_content --all-features`,
`ambition_app --test repro_walls` and `ambition_platformer2d_actor_monolith
--lib`) do not run either demo app crate — but `./run_tests.sh`, the repository
default, plans `cargo test --workspace --no-fail-fast`, both crates are workspace
members, and `ambition_demo_smash_app` declares `[[test]] name = "smash_it"`, so
the backbone DOES run them. ⇒ this is a testing-CADENCE decision, not a
disconnected test target to reconnect.

```text
ambition_content        content_it     green    (CI runs it)
ambition_workspace_policy  policy      1 red    (CI runs it — so CI was red too)
ambition_demo_twintrack_app twintrack_it 1 red   ⛔ backbone only — not the per-turn gate, not CI
ambition_demo_smash_app  smash_it      5 red    ⛔ backbone only — not the per-turn gate, not CI
```

✔ **both red ones were fixed 2026-08-19, neither was a physics bug.** The policy
rule required the literal `pub use plugin::{PortalPlugin` — it pinned the
symbol's POSITION IN A BRACE LIST, so gaining `PortalGunPlugin` sorted it out of
first place; it now takes two needles and survives any ordering. TwinTrack's was
geometry: the 2026-08-12 plaza relayout put the light-tagger five pixels off the
laboratory's eye height, collapsing the SR-3 lead angle to 0.3°; restoring the
tagger's original 150 px vertical offset took the separation to 0.05–0.074 rad.
The smash five are decision §23, not a repair.

⇒ ▢ **the open question is whether these two crates get a CI job.** ⛔ do not
add one reflexively — the demo crates link Bevy, and the backbone's whole cargo
job is already 607s; the honest options are (a) a job per demo app crate, (b)
one job running `cargo test -p ambition_demo_smash_app -p
ambition_demo_twintrack_app`, or (c) accept that the demo crates are covered
only by the backbone and say so out loud where someone will read it. ⭐ the
rule that a guard nobody runs is not a guard — this pair had been red since
2026-08-12 and 609063bc1 respectively, and a passing fast gate said nothing
about either.

### ▢ Two things found in passing 2026-08-15, logged rather than fixed

**1. ⚠ TWO WORLDS FAIL LDtk VALIDATION TODAY, and the tool writes them anyway.**

⭐⭐ **MEASURED 2026-08-17 — it is ONE world, not two; the count depends on WHICH
PATH you validate.** The demo path (`game/ambition_demo_mary_o/…`) is a SYMLINK
into `ambition_map_assets`, and the sidecar manifest a world validates against
(`<world>.entities.json`) resolves strictly BESIDE the path you name, sitting
next to the symlink, not the real file — so validating the raw copy invents 26
`MaryOBlock` errors that don't exist through the canonical path. Mary-O is
CLEAN; its manifest declares `MaryOBlock` and always did. Only `sandbox.ldtk`
genuinely fails, with 4 false-positive errors: cross-world `LoadingZone` targets
the single-file validator cannot see into a sibling world.

▢ **the fix is cross-world resolution (or a documented suppression) for four
edges, and nothing for Mary-O.** ⛔ do NOT hand-write an entities manifest for
the map_assets copy: that file is *"the same shape `def register-entity --spec`
consumes"*, i.e. what editor definitions are GENERATED from, so a second copy
is a fork of an authoring source.

⚠ invocation, because getting it wrong looks like success:
`PYTHONPATH=tools/ambition_ldtk_tools python3 -m ambition_ldtk_tools validate <world>`
— the package is not installed in this environment, so a bare `python3 -m …`
prints *"No module named"* and exits 1, which a naive error-count reads as
clean. ⚠ its diagnostics are INDENTED, so `grep -c '^error:'` returns 0 on a
failing run. ⛔⛔ the errors do not block the write: three `error:` lines filled
a `| head -3` and hid the `wrote` line under them, so a landed edit was
reported as refused.

⇒ the row that owns the instrument is D163 (which also carries the retracted
"duplicate pirate spawns" finding — ⛔ coincident rider/mount pairs are
AUTHORED and must not be deduplicated). ⛔ do not re-derive the counts here.

**2. ◐ THE SMASH LANE'S VISUAL FIXES HAVE NOW BEEN PHOTOGRAPHED — what is left
is Jon's eye, not a capture.** Corrected 2026-08-17: the camera-close ease, the
3-2-1-GO card and the winner card have all been captured through the shipped
shell (D130 gave `capture_scene` `--press touch:XxY`; two-CPU matches
photographed 2026-08-16 and 2026-08-17), the winner card is verified naming the
fighter, and the countdown is verified drawing. ⇒ what genuinely remains is one
judgement, not an instrument: whether 5 Hz is the right close rate — a number
that was chosen, not measured. See D128's ACTIVE TRUTH block for the one
outstanding product-acceptance item; ⛔ do not dispatch another capture to
establish status.

⭐⭐ **THE SPRITE/BOX CLUSTER HAS A MEASURED SHAPE.** Three reports (snake too
big · player hurtbox mismatched · Sanic too small) are one question: there are
TWO sizing roads and a body's size depends on which one it is on.

```text
published road   collision derived from the sprite's own body_metrics,
                 quad size stated explicitly by ActorRenderSize
legacy road      collision * collision_scale, a hand-tuned per-character
                 number in character_catalog.ron ranging 1.15 .. 2.1
```

`ActorRenderSize`'s own doc says it: *"Absent ⇒ the actor uses the legacy
`collision_scale` render path."* ⭐ 194 of 196 spritesheet specs already publish
`body_metrics`, so the DATA is not the gap — the gap is which road a character
is wired onto.

✔✔ **A separate divergence — twelve sheets whose `authored_body` differed
between full resolution and reduced quality tiers (`player_extended`, three
`player_*_review` sheets, eight `robot_*` variants) — is CLOSED 2026-08-19.**
All twelve re-rendered at full resolution and their tiers refreshed; every one
now agrees across all four publications. The guard
`a_sheets_gameplay_body_does_not_depend_on_the_graphics_setting` (in
`ambition_sprite_sheet`) asks the BAKED INDEX every runtime lookup reads and
compares each full-res target's `authored_body` claim against all three tiers;
it was RED on the twelve before the regen and green after, and its zero floor
is poison-verified — point `TIERS` at names nothing publishes and it refuses at
*"only 0 sheet/tier pairs were compared"* rather than reporting agreement.

✔✔ **THE ADOPTION COUNT (2026-08-16): TWO.** The decision site is
`posed_body_for` in `character_runtime/presentation.rs` — a definition
authoring `BodySource::SpriteAuthored { world_per_pixel }` gets
`SpritePosedBody`, and `sync_sprite_posed_bodies` keeps its collision box,
sprite quad and quad offset all derived from the sheet, *"so none of the three
can drift from the other two."* `BodySource::Explicit` and `None` fall to the
legacy path.

```text
with_sprite_authored_body callers   2   (player_robot_lineage, mary_o)
character_catalog.ron rows with a hand-tuned collision_scale   33   (1.1 .. 4.5)
sheets publishing the body_metrics the good road needs        194 of 196
```

⭐⭐ `world_per_pixel` IS the common unit Jon's hurtbox note says was never
established.

⛔ **do not report an absence from a windowed search** — a 2026-08-16 count
here that searched only the first 400 characters after `body_metrics: Some(`
claimed 6 sheets declared `authored_body: true` and that `player_robot_v3` was
not one of them; both halves were false — the real count is 37 (including v3,
`robot`, `player_robot_v2`, noether, alice, bob and 30 more), and the flag sits
2,070 characters into v3's record. The player's call site fires today.

✔ **the player hurtbox report was already fixed** (renderer `dd744b4`,
*"v3 authors his collision box instead of measuring the idle alpha bbox"*):
authored body 57 × 91 against a drawn idle silhouette of 71 × 103, 0.71× the
area, against Jon's reported 1.28× wide / 1.29× tall for the old box.

✔✔ **BUT NOTHING COULD SEE THAT, AND THAT GAP IS NOW CLOSED** (`4213db3d4`).
Two existing tests pinned the box (standing height, hurtbox) but poisoning
`body_pixel_bbox` back out to the full silhouette left BOTH GREEN. The new test
compares the two rectangles data-to-data — the atlas packer already trims every
frame to its opaque bbox, so the union of a row's `off`/`w`/`h` IS the
silhouette, in `body_pixel_bbox`'s own pixel space, no PNG decode, survives a
redraw. An untrimmed frame reports the whole 256×256 logical frame and would
wave through a body box of any size, which is what the vacuity guard catches.

⚠ **the snake and Sanic genuinely do not declare an authored body**
(`solid_snake`, both `snakes_on_a_*`, `sanic`, `super_sanic` all measured; only
Sanic's two PROPS are authored) — the two reports are the same bug and the fix
is the same three-line edit in each renderer target. ⛔ **do not fix Sanic's
scale in isolation** — a fourth hand-tuned constant is what this cluster is
made of. ⛔ do not delete `collision_scale` before counting: a shipped
capability can have zero adopters, and so can a legacy path still carrying half
the roster.

✔ **D172 — body-vs-body contact did not exist, so a closing fighter's reward
was to sail past the one it was closing on.** Built by `da884be08` as
`ambition_platformer2d_core::movement::body_contact`: a constraint on the motion
a body PROPOSED, applied immediately before the world sweep resolves it. Monotone
and position-free, so nothing is ever teleported apart. `BodyContact { resistance }`
is presence-as-opt-in and the smash stage grants it to its cast; the engine does
not know the word jostle. Guarded by
`movement::kernel::tests::a_grounded_body_walking_into_another_one_is_stopped_by_the_real_sweep`
(through `step_motion`, against the `approach()` overwrite that erased the force
version) and `the_stage_kills::the_stage_grants_body_contact_to_both_seated_fighters`.
Schema v59 → v60.

⭐⭐ **AND IT TURNED SMASH GREEN: `smash_it` 26/7 → 34/0.** All seven long-red
guards, both repertoire ones and all five `the_stage_kills`. That MEASURES §23's
hypothesis — *"the limit cycle is very plausibly exposing a missing physical
spacing primitive rather than a brain defect"* — under the corrected ruleset,
where the original diagnosis was taken in a match running no smash rules at all.
Nothing in the brain moved.

⛔ **three rules it had to learn, each from a red test, each worth keeping:**
only motion that DEEPENS an overlap is resisted (resisting every direction left
four fighters spawning on one point unable to walk apart); a step longer than one
tick of the body's own WALK is a launch and passes through untouched (contact ate
knockback and three guards about matches ENDING went red); and the blockers come
from a snapshot taken before ANY body integrates (otherwise query order decides
who wins the contest, which is a desync).

▢ **the resistance NUMBER is Jon's**, and airborne contact is deliberately not in
this slice. `BodyContact::FIRM` is 0.85 — two fighters walking into each other
stall where they meet and a determined one still squeezes past — chosen because
the genre does that, not because anything measured it.

✔ **D173 — a worktree agent's goal guard judged the MAIN checkout.** `repo_root()`
was always right (it resolves through `__file__`); the HOOK COMMAND took
`${CLAUDE_PROJECT_DIR:-$PWD}` unconditionally, and a session that started in main
and then entered a worktree still carries that pointing at main. Fixed by
`scripts/goal_guard_hook.sh`: `$PWD` wins when it is the SAME REPOSITORY as the
declared root (a worktree shares `--git-common-dir`; a nested repository does
not), and the declared root wins otherwise — so the 2026-08-05 failure where one
`cd` into a nested repo silently released a 72-hour run stays closed too.
Guarded by `scripts/tests/test_goal_guard_hook.py`, whose fixtures are a real
`git worktree` and a real nested repository, and each of whose two tests was
falsified by poisoning the resolver to the other extreme.

⚠ **the fixed behaviour is that an unarmed worktree reports NO GOAL**, which is
the model `goal_guard.py` already documents: a worktree reads its own `.goal/`,
which is gitignored and therefore absent until somebody arms one. Use `--share`
when two lanes are working ONE run.

⚠ **`.claude/settings.json` keeps the old command as a FALLBACK** — a checkout
without the resolver still runs the guard. Breaking that hook takes the whole
standing-goal mechanism down, and it can only be exercised by ending a turn,
which is why the logic moved into a script a test can run.

- ▢ **D171 — THREE MORE DOCS CARRY OPEN ITEMS NO LEDGER ROW CAN REACH.**
  (promoted 2026-08-20)

The same sweep that produced D170 found seventeen planning docs reachable from
no ledger row, intake or map. **Fourteen of them are reference with no open
work** and are correctly left alone — a doc is only stranded if it holds WORK.
These three hold some:

| doc | open | what |
| --- | --- | --- |
| [`engine/character-actions.md`](engine/character-actions.md) | 4 | cast-action authoring where a body still relies on defaults · presentation metadata on authored moves · two DECISIONS deferred until a real repertoire forces them |
| [`engine/unified-movement-kernel.md`](engine/unified-movement-kernel.md) | 2 | block ↔ chain crawl transfer — ✔ VERIFIED OPEN 2026-08-20: `CrawlAttachment::Chain` returns early into `crawl_chain` and `Block` falls through to the riding path, so the two are separate roads with no shared transfer rule · portal transit inside authored gravity zones (its own text says there is no known bug and no room authors the combination — customer-gated, leave ▢) |
| [`demos/super-mary-o.md`](demos/super-mary-o.md) | 1 | crossing 1-2 while GROWN, plus further authored levels. ⚠ its second row was RETIRED 2026-08-20 — it claimed nothing had ever bonked a ?-block while grown, which the ✔ two lines above it contradicts |

⛔ **THESE ▢ WERE COUNTED, NOT CHECKED — EXCEPT WHERE THE TABLE SAYS OTHERWISE.**
One is now verified open (see the movement-kernel row); the rest were counted, and a row's prose goes stale faster than the code — a ▢ on
work that already landed has cost this project four sessions. Grep for the thing
each says is missing BEFORE working it, and if HEAD contradicts the doc, update
the doc.

⚠ two of `character-actions.md`'s four are explicitly *"decide only when a real
repertoire exceeds prompt capacity"* — they are waiting on a customer, not on
effort, and should stay ▢ until one exists.

- ▢ **D170 — IMMUTABLE CONTENT / TRANSACTIONAL CONSTRUCTION HAS SIX OPEN ITEMS
  AND NO LEDGER ROW.** (promoted 2026-08-20)

⭐ **PROMOTED, NOT WRITTEN.** [`engine/immutable-content-and-transactional-construction.md`](engine/immutable-content-and-transactional-construction.md)
was verified against `fda5db88` on 2026-08-19 — the day before this promotion —
and is reachable from two ADRs, a concepts doc and a related-work doc, but from
**no queue row**. That is the same shape as the seven Engine 1.0 plans stranded
on 2026-08-14: designed frontier, structurally invisible to the execution
authority. The work is already specified there; this row is the pointer.

⛔ **the biggest one is the old operation 5**: there is still no production
cross-room snapshot caller exercising source-snapshot selection, decode/
compatibility rejection BEFORE mutation, rollback entity identity and remapping,
restoration of non-room authoritative state, and atomic commit. Room-transition
use of `RoomConstructionPlan::apply_to_world` does not prove it.

⚠ two of the six are deliberately NOT actionable yet and should stay ▢ rather
than be worked: corrected-input cancellation and peer-coordinated lifecycle
commit belong to real external netplay, because local sync testing cannot
mispredict. Its own doc says so; do not build a synthetic local ritual for them.

The remaining three are external-consumer proof — run the visible consumer on a
machine with a display, measure first-room workflow and deliberate-error
diagnostics rather than describing them, and exercise authoring from a second
meaningfully different consumer before freezing a public prefab/content API.

⚠ **this row is a POINTER and the doc is the authority** — update the doc, not
this row, and keep the row's claim to "there are open items" so it cannot rot
into a stale summary of them.

- ▢ **D167 — THE LIVE-STATE ↔ PERSISTENCE ↔ ROOM-CONSTRUCTION BOUNDARY. Two
  legs closed 2026-08-20; two open.**

Jon's architectural review of `4af278e77`, 2026-08-19. Its headline is that the
custody work found a boundary worth making crisp, and that this pays off across
items, possession, vehicles/mounts, save/load and future body-carry mechanics.
⛔ he also asked for **no new large architectural campaign** until the two closed
legs below were settled, and for **no GGRS-sized carve and no Smash AI tuning**
in the same turn.

✔ **THE CUSTODY SCHEDULING BOUNDARY IS STRUCTURAL.** `InCustodyOf` has two
owners in different domains — the item road reprojects it onto objects, and
`project_driven_body_custody` owns the whole non-item body population — and the
item road READS what the body derive writes. The two chains were internally
correct and unordered siblings under `PlayerSimulation`. Fixed by
`lifecycle::BodyCustodySettled`, a label the derive carries and
`ItemPickupSet::CoreHeldItems` orders against — the CAPABILITY, not the
feature, so the edge survives body custody leaving the possession ability.
Guarded by `the_occurrence_ledger_learns_of_a_driven_body_on_the_tick_it_is_driven`.
⚠ measured first, and it was not broken: with no edge at all the derive already
ran first, because unconstrained siblings fall out of the topological sort in
plugin-add order. ⛔ **the poison is the REVERSED edge, not the missing one** —
deleting it leaves everything green; `.before(..)` reddens three tests.

✔ **A RELATIONSHIP MAY NOT CROSS THE DURABLE HORIZON WITHOUT ITS AUTHORITY.**
Possession-derived body custody was reaching the save file: the mirror queries
the generic component, so a possessed body answered it, and the file said
*"this enemy is in somebody's hands"* while `PossessionState` — the authority
that makes it true — is rollback state and is not saved. It never failed live
(the projection republishes the custody leg every tick, retracting the row
before any room build acts on it). `persist_occurrence_horizon_to_save` now
writes an `InCustody` claim only for occurrences whose custody the durable road
can RESTORE (the item road, spelled `With<ItemCustody>`), so a body's
occurrence is simply absent and its room authors it on load. Guarded by
`a_save_taken_mid_possession_does_not_delete_the_enemy_in_a_fresh_process`,
whose poison reports zero bodies behind the identity: the enemy deleted from
the world, permanently. ⛔ **the guard that was CLAIMED and not written**:
`37ba867` said it proved this; its test assigns `PossessionState::default()`
into the running world and steps once, which is a narrower property, not a
fresh process.

✔ **BODY-CUSTODY PROPAGATION HAS LEFT THE POSSESSION ABILITY** (`cb1aa427d`),
moved unchanged to `body_custody::project_body_custody`; possession supplies one
ROOT and the closure below it is shared, so a carry, a vehicle or scripted
transport joins beside it instead of editing an ability. ⛔ still concrete and
typed: no registry, no erased callback, no generic attachment graph.

▢ **(the case, kept because it is the reusable half.)**
`project_driven_body_custody` calls itself *"the one owner of `InCustodyOf` for
BODIES"* while living in `abilities/traversal/possession.rs`, and it now closes
custody transitively over mounts (`RidingOn`), limbs (`Limb`) and arbitrarily deep
attachment chains. Possession is one ROOT REASON a body stops being resident, not
the law governing every attachment: a carry, a vehicle, scripted transport or
room-capable capture would each have to modify the possession ability to
participate, which is feature-centric ownership creeping back. ⇒ move the
mechanism to a neutral actor lifecycle/body-custody module. ⛔ **no registry and
no generic graph framework** — keep it concrete and typed, understanding the few
attachment relations that actually exist. `BodyCustodySettled` already points at
the system rather than at possession, so the move costs no reader.

✔ **ROOM CONSTRUCTION-LANE ORCHESTRATION IS ONE COMPOSED VALUE** — the capability
lanes travel as `capability_lanes::CapabilityLanes`, a plain struct with named
fields whose every operation destructures `Self` exhaustively. `spawn/mod.rs`
named the two lanes **48 times before and 4 times now**, 1209 → 1048 lines. The
poison — a third field — produces **seven compile errors**: `E0063` at
construction plus `E0027` at each of `claim_planned_ids`,
`write_deterministic_dump`, `debug_assert_binding`, `commit`, `verify`,
`respawn`. ⛔ no `Any`, no `TypeId`, no registry, no service locator: each
operation is a GENERIC function over `ConstructionDomain` applied once per
field. ⚠ `Services = ()` is a BOUND on that set, not a coincidence — the actor
lane reads frozen catalogs at execution time and is composed BESIDE the
capability lanes rather than inside them, so a future capability that needs
services fails the bound instead of quietly joining.

▢ **(the case, kept because the MEASUREMENT is the reusable half.)** Gravity was
the right second customer and the extraction validated the federation design; the
measurement is what it cost `RoomFeatureConstructionPlan` — roughly ELEVEN
enrollments (plan field, receipt field, preparation, predicted roster, plan
construction, deterministic dump, binding agreement, verification, single-root
reconstruction, commit, committed roster union), and the portal lane repeats most
of the same shape. ⇒ introduce one explicit typed composition owner —
conceptually `RoomConstructionLanes` — owning planned ids, the deterministic dump,
binding agreement, commit, verify, respawn-by-`SimId` and receipt aggregation.
⛔ **a normal concrete Rust type: no `Any`, no executable registry, no `TypeId`,
no service locator.** The goal is for the plan to know it HAS lanes rather than
reimplementing every operation per family. ⛔ **do not extract a third or fourth
construction family first** — that is what makes this evidence-driven rather than
speculative. ⚠ gravity is a successful construction-VOCABULARY extraction, not a
fully extracted domain: its construction lives in `shared_tangle` while its
scheduling/runtime ownership is still in the actor monolith.

▢ **SMASH: THE MEASUREMENT STANDS, THE ASSERTION TEXT DOES NOT.** ⚠ TAKEN by the
smash lane 2026-08-20 (`smash-parity`), assertion text only. "One mind
played twice" is FALSIFIED — the CPUs have different RNG streams, draw different
samples, fight, overlap attack range, create live hitboxes, land hits and take
mirrored outcomes; at difficulty 5 the execution-noise effect is 0–1 frames,
nowhere near enough to break a symmetric initial condition between two agents
running the same deterministic policy on symmetric observations. ⛔ **do NOT alter
symmetric spawn placement or add stronger randomness to satisfy the old
behavioural tests** — that is a gameplay/fairness decision and it is Jon's. ⇒ the
only work here is the ASSERTION TEXT: a guard may stay red pending a product
decision, but its failure message must not diagnose a mechanism already disproved.

⚠ **the foreign-room release policy remains a product decision** and it was
correct not to invent one: a body released away from its authored room lives there
until that room unloads, after which its authored record recreates it at home.
*"Leave this actor permanently where I released it"* would need body `Placed`
whereabouts plus reconstruction relocation support.

- ▢ **D169 — EVERY GAME BUILT ON THIS ENGINE CARRIES A PLATFORM-FIGHTER NOUN.**

Design in
[`engine/world-geometry-and-spatial-semantics.md`](engine/world-geometry-and-spatial-semantics.md).
⭐ promoted 2026-08-20 after re-running the stranding measurement across every
`.md`, `.rs`, `.py`, `.sh`, `.toml` and `.ron` in the tree (34,790 files): 1 of
267 planning docs was referenced by nothing, and this was it. ⇒ re-run that
census, do not trust the list.

⭐ **MEASURED 2026-08-20, and it is worse than the plan says.** The plan cites
three fields in `platformer2d_core::World`:

```text
:888  blast_margin: f32           + a serde default and DEFAULT_BLAST_MARGIN
:900  side_blast_margin: Option<f32>
:906  ceiling_blast_margin: Option<f32>
      three builders, an LDtk lowering pass, a render overlay
```

The repository names them **206 times across 14 crates and games** — including
`ambition_demo_mary_o` and `ambition_demo_twintrack`, neither of which is a
platform fighter. Mary-O's `World` has a `blast_margin`.

⇒ **the generic fact underneath is a boundary region with a CONSEQUENCE.** Smash
calls it a blast zone and loses a stock; Mary-O calls it a pit and respawns;
Ambition calls it out of bounds. Engine owns the geometry, the game owns the
meaning — the plan's own principle 1.

⛔⛔ **BUT THE MECHANISM IS ALREADY GENERIC, AND THE PLAN NAMES THE WRONG LAYER.**
`apply_world_hazard_gate` (`platformer2d_core/src/movement/kernel.rs:422`)
computes a per-axis distance past the world AABB and emits
`ResetCause::LeftTheWorld`; *"policies flag; the body's owner applies its reset
policy."* The consequence is already the game's. `blast_margin`'s own doc says
so: *"a platformer's pit depth and a platform fighter's blast zone — the same
number, and it belongs to the STAGE."* ⇒ there is no bespoke platform-fighter
PRIMITIVE to remove. What is genre-specific is the WORD.

⭐ **the word's load-bearing home is the AUTHORING SCHEMA, not the struct.** The
LDtk converter reads the authored key by that name (`level_field("blast_margin",
..)`), and **all SIX shipped worlds carry all THREE fields in
`defs.levelFields`** (`sanic_speedway`, `intro`, `sandbox`,
`you_have_to_cut_the_rope`, `hall_of_characters`, `mary_o`). ⭐⭐ ZERO levels
author a VALUE — 18 schema entries, no data behind any of them, so the rename
costs **no content migration** — only a schema rename in files the LDtk editor
owns, which is why the authoring half is Jon's call and is written up in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §26.

⛔ **do NOT do the Rust half alone.** The struct field and the authored key are
one name; renaming one and not the other needs a mapping, and a mapping is the
shim this project refuses. It is one change or it is not worth 206 sites.

⛔ **the error the plan itself records, because it is the cheap kind to repeat:**
its own 2026-08-15 triage tested trigger #1 of five, found it negative, and
generalised to all five. *"The measurement was sound; the sentence after it was
not. A negative on one trigger is not a negative on the plan."*

⚠ **`BlockKind` is the plan's other half and is NOT this slice.** That enum mixes
contact law, traversal permission, world consequence and contact affordance on one
axis, and the diagnosis was re-measured as correct — but its trigger has not
fired. Take the blast zone first; it has a customer, and it removes rather than
adds.

- ▢ **D168 — CONTROL AUTHORITY AND AI POLICY ARE TWO FACTS IN ONE COMPONENT.**

Design and measurement in
[`engine/control-authority-and-ai-policy.md`](engine/control-authority-and-ai-policy.md).
Jon's 2026-08-19 review named this as a broad direction to resume once the custody
and construction legs of D167 closed; all four of those are closed.

⛔⛔ **THE REVIEW REFUSED THE OBVIOUS VERSION FIRST, and that is the load-bearing
half**: `Brain::Capability(BrainId)` plus registered executable dispatch *"removes
closed enum edges by adding a service locator"*. No `Any`, no `TypeId`, no
`BrainId`, no registry. Same prohibition as `CapabilityLanes`, same reason — an
erased id trades a compile error for a runtime lookup.

⭐ **MEASURED 2026-08-20**: `Brain` is 2 variants and `StateMachineCfg` is 12;
`Brain::Player` is named **194 times across 14 crates/games**, `Brain::StateMachine`
107; 13 exhaustive matches; and **8,950 non-test lines of platform-fighter policy
(`brain/fighter`, `brain/smash`) sit inside `ambition_characters`**, a floor crate
every composition links — because policy shares a component, and therefore a
crate, with control authority.

⇒ two typed components, neither erased: `ControlAuthority` (generic, the
participant slot a body reads) and a domain-owned `AiPolicy`. Possession then
INSERTS control authority and leaves policy alone, which retires
`PossessionState::restore_brain` — today a rollback-registered field that
round-trips an entire AI policy's runtime state through a resource whose subject
is *who is driving*.

⛔ **the first slice is the SEAM, not the migration** (the review said so twice:
*"evidence-driven carve; do not redesign the brain stack at once"*). Introduce the
component, move possession onto it, delete `restore_brain`; nothing changes crates.
Only then is the Smash/Fighter move priced by measurement — the way gravity priced
the construction federation.

✔ **THAT SLICE LANDED, 2026-08-19 → 2026-08-20** (`1a9f3a372`, `bbf02bf47`).
`ambition_characters::brain::DrivingParticipant(PlayerSlot)` is control
authority; `Brain` is AI policy and nothing else — `Brain::Player` is gone from
executable code, so the 194 sites and the 13 exhaustive matches are settled, and
`PossessionState::restore_brain` is deleted because a possession no longer takes
a policy away to give back. `project_driving_participant` is the ONE runtime
writer. Rollback schema v59; the seat is registered AND value-probed, because two
peers can agree a body is driven and disagree about by whom.

▢ **WHAT IS LEFT IS THE CRATE CARVE, and it is priced but not started.** 8,950
non-test lines of platform-fighter policy (`brain/fighter`, `brain/smash`) still
sit inside `ambition_characters`, a floor crate every composition links. The
reason they were there — policy sharing a component with control authority — is
gone, so the carve is now an ordinary move rather than a coupling.

⚠ **`Brain` is a ONE-VARIANT enum now** (`StateMachine(StateMachineCfg)`).
Collapsing it to a struct is a separate decision and deliberately not taken here:
the enum is where a future non-state-machine policy attaches, and flattening it
would be a rename campaign that buys one indirection.

⚠ **naming residue, and it can wait** (GPT 5.6, 2026-08-20): `DrivingParticipant`
lives under `characters::brain`, which is the module it was carved OUT of. That
is a module question, not a coupling — do not reopen this row for it.

- ▢ **D117 — Finish the controlled-character actor kernel. UNBLOCKED 2026-08-17:
  the decision it rested on is ANSWERED.**

⭐⭐ **THE CONTROLLED-BODY INTERACTION SEAM IS FINISHED — 2026-08-19.** Two
breaches were invisible in single player: the POSE was written unconditionally
to whatever carried `PrimaryPlayer`, so under possession the possessed body
opened the chest while the vacated home avatar played the reach-and-open; the
PRESS was read from and cleared on SLOT 0 whichever seat was driving, so a
second seat's interaction spent seat 0's buffered interact. The authority is
the BRAIN — a seat that possessed an actor carries `Brain::Player` on that
body — so `crate::control::ActingParticipant` asks that once and answers both
questions from it, stating the primary-seat startup fallback in ONE place
instead of four call sites. The pose now lands on the acting body when it has
`BodyAnimFacts` and on nothing when it does not.

⭐ **two more control-seam breaches closed the same day.** `open_death_interlude`
inserted `ScriptedControl` directly, a marker that is DERIVED from
`ControlHolds` being non-empty; a captor releasing a fighter that died in its
grip found an empty claim set and took the marker off a corpse mid-interlude.
Death now claims `ControlHold::Sequence`, sharing that bit with the flagpole
slide, the goal brake and the act clear — states the SAME body cannot also be
in.

`SlotInteractionState::get_mut` CLAMPED an out-of-range `PlayerSlot` onto the
last valid one, so `PlayerSlot(9)` and `PlayerSlot(3)` were the same
controller. It returns `Option` now; `get` still answers `default()` for an
out-of-range READ, a different, defensible question.

⛔⛔ **THE TEST TIER LEARNED THE SAME LESSON NINE TIMES.** Nine fixtures each
carried their own `ScriptedStick`, apply system and copy of a comment about the
schedule; five guessed `PreUpdate`, where the participant pipeline overwrites
the write before the sim sees it. One `ambition_platformer2d::scripted_input`
seam states the ordering once (after `InputSet::Route`, before
`accumulate_control_frame_latch`) and ships the falsifier with it:
`ScriptedControlsObserved` counts what the sim's own slot table carried, and
`assert_the_script_reached_the_simulation` pairs the negative test with its
negative. ⇒ **`spikes_spend_rings` had been GREEN SINCE 2026-08-09 for the
wrong reason** — a `#[cfg(feature = "input")]` fork read the demo crate's
feature while the thing erasing a direct `ControlFrame` write is
`ambition_platformer2d/input` in the DEPENDENCY; under the prescribed per-crate
command that composition was never selected. ⭐ **the lesson generalises: a
`#[cfg(feature = ...)]` in a TEST names THIS crate's feature, and the
behaviour it forks on usually belongs to a dependency.**

⭐⭐ **the blocker is gone.** This row waited on the hit-emphasis / proper-time
question (`awaiting-maintainer-decision.md` **#6**), and Jon ruled it
2026-08-17: hitlag freezes the BODY that is in it, on both roads. ⇒ the
movement/TIME integrator fork is executable now, as is folding the three
per-population `decay_reaction_timers` calls into one.

⭐ control authority CONVERGED (one `tick_controlled_brains`), and
`tick_actor_brains` reads as a sequence after three extractions.

✔✔ **THE STEP ITSELF IS ONE SEAM NOW (2026-08-18).** Both roads reached
`ae::step_motion` by writing the same two lines beside their own call —
refresh the axis params from the resolved tuning, then zero `dt` if the body
is in hitlag — duplicated exactly the way D114 happened (a hit between two AI
bodies froze neither, because the freeze was a line one road had and the other
did not). Those two steps are now `ambition_characters::actor::step_body`,
taking the BODY rather than a `dt` a caller can compute wrongly:

```text
before   avatar/body_integration.rs   axis params · hitlag · ae::step_motion
         features/enemies/integration axis params · hitlag · ae::step_motion
after    both                         → actor::step_body(.., combat, tuning, ctx)
```

⛔⛔ **THE "FOLD THE THREE `decay_reaction_timers` CALLS" ITEM IS REFUSED WITH
CAUSE.** They iterate different populations in different phases; what they
fork on is the CLOCK, and that fork is correct:

```text
actor tick    world_time.sim_dt()   scaled — slows with bullet-time
boss tick     world_time.sim_dt()   scaled
controlled    time.delta_secs()     RAW — and this is DELIBERATE
```

⭐⭐ **because HITSTOP IS A `sim_clock` REQUESTER.** A connect asks the sim
clock down, so decaying `hitstop_timer` on `sim_dt()` slows the timer that
ENDS the freeze by the freeze itself, stretching i-frame and hitstun windows
measured against the same scale. ⇒ i-frames are a promise to the player in
REAL seconds; a bullet-time moment must not hand out longer invulnerability,
the same reason the double-tap windows are unscaled.

⛔⛔ **AND THE `Res<Time>` WAIVER SAID SOMETHING FALSE, WHICH IS HOW THIS
HAPPENED.** It claimed *"the reaction timers still compute their own scaled dt
manually"* — no such scaling exists or should. Reading the false sentence and
"correcting" the code to match it is what cost seven boss tests
(`boss_contact_iframes`, `boss_lifecycle`, `boss_motion_parity`) before they
refuted it. ⭐ a false justification does not mean the decision under it is
false. The waiver now carries the real reason, and
`the_reaction_timer_clock_forks_on_purpose` pins BOTH sides — a fork guarded on
one side only drifts back.

⭐ **verified NOT a determinism defect**: `BodyCombat` is rollback-registered
and this decay runs in the sim schedule, but production pins
`TimeUpdateStrategy::ManualDuration` to the sim tick whenever rollback
participants exist, so the raw delta was deterministic — a different clock,
not a desync.

⭐ **placement follows the destination's contract**: `ambition_characters` says
its job is *"the same brain + control-frame contract drives players, NPCs,
enemies, and bosses"*, and a body's hitlag IS body identity, so core cannot
host it. ⛔ it deliberately did NOT land in the monolith — Jon: *"try not to
dump things into it to make the problem worse."*

⚠ guards are a PAIR and the pair is the point:
`a_body_in_hitlag_does_not_travel_through_its_own_freeze` plus
`and_the_same_body_travels_once_the_freeze_clears` — a body that never moves
passes the first for the wrong reason. Poison-verified: deleting the branch
walks the frozen body 8.98px and leaves the control green.

✔✔ **AND THE HOME ROAD WAS REBUILDING THE COLLISION WORLD PER BODY.**
`integrate_sim_bodies` composites `world_with_sandbox_solids` once per frame
for the actor loop, and the home road called it AGAIN, per body, from
identical inputs. Both roads take the one composited world now, and
`integrate_home_body` loses three parameters. ⭐ the deeper win: two composite
sites is two places for moving platforms, gate solids, water and portal carves
to drift apart.

⭐⭐ **THE ANSWER TO "SHOULD THE TWO INTEGRATORS BECOME ONE FUNCTION?" IS
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

⇒ fusing the last two would build exactly the god function this milestone
forbids (*"no replacement god `ActorContext`/service bag"*). ⛔ so
`integrate_home_body` STAYS: the two roads live in disjoint queries
(`With`/`Without<PlayerEntity>`) with different cluster shapes and cannot
share one Bevy loop anyway.

⭐⭐ **THE PROPERTY THAT ACTUALLY MATTERS IS TRUE NOW, AND IT IS CHECKABLE:
production has ZERO direct `ae::step_motion` calls.** Every body — home,
actor, seated fighter, boss — reaches the movement kernel through
`step_body`; the only two remaining spellings in the monolith are both inside
`#[cfg(test)]` helpers.

✔✔ **THE FOOTPRINT IS ONE RULE NOW, AND THE BOUNDARY THAT REFUSED IT WAS
STATING A FALSEHOOD.** `publish_body_footprint` is the single publish; both
roads call it, and the actor road's coarse-envelope override became a
PARAMETER rather than a species. ⛔⛔ `attack_geometry`'s header said *"this is
boss-attack-specific geometry only"* — which turned a correct move into an
obviously-wrong one at zero cost, except it was FALSE, measured:

```text
collision_aabb / SimpleActorGeometry — production call sites
  home body footprint publish        avatar/body_integration.rs
  actor body footprint publish       features/ecs/actors/update.rs
  the debug overlay                  game/ambition_app/src/dev/…/gizmos.rs
  boss callers                       ZERO
```

⇒ ⭐⭐ **a stated boundary is only worth what its accuracy is worth.** The
header now says what the module actually holds, and records the measurement
so nobody re-derives it.

▢ **THE CARVE IT IMPLIES IS REFUSED FOR NOW, WITH CAUSE AND A SIZE.** The
universal half of `attack_geometry` wants to live below the boss crate beside
the other body vocabulary — but `CombatGeometry` names `ActorSpriteMetrics`
and `AnimationSelection`, both boss-crate types, and the edge runs
`boss_encounter → characters`, so `ambition_characters` cannot reach any of
it. ⇒ moving the trait means moving three things, not one: a D33-shaped
slice, not a file move. ⭐ unifying the publish first makes that carve
strictly smaller — one call site to move instead of two.

▢ **AND ONE THING THIS FOUND ON THE WAY, MEASURED RATHER THAN ASSERTED: A
POSSESSED FLYER CANNOT REACH ITS OWN TOP SPEED.**

A possessed body does not change roads — possession is brain transfer, so the
body keeps `Without<PlayerEntity>` and stays on the ACTOR road with
`Brain::Player` driving it. That road's flight limb OVERWRITES the input axes
with the brain's `velocity_target` projected onto the frame and normalised by
`flight_speed`:

```text
brain/player.rs:120   velocity_target = stick_local → world × max_run_speed
integration.rs:~350   axes = (velocity_target → local) ÷ flight_speed
                      flight_speed = max(chase_speed, max_run_speed, 1.0)
⇒ a fully deflected stick reaches max_run_speed / flight_speed of the available
  deflection — full only while chase_speed ≤ max_run_speed
```

⭐ steering WORKS (the round trip is local → world → local), and only the
MAGNITUDE is wrong: a human possessing a body whose `chase_speed` exceeds its
`max_run_speed` flies it at a fraction of what the same body does under AI.
⚠ latent on the shipped cast — only two catalog rows author `chase_speed` at
all, and no flyer among them — so this is a model defect rather than a live
one, fixed when the flight limb is next touched rather than chased now. ⭐ it
is exactly the milestone's own sentence made concrete: *"the protagonist
should be special because of current control assignment … not because generic
simulation has a hidden coordinate system."* Here the hidden coordinate system
belongs to the AI.

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
local view). ✔ the SELECTION shipped 2026-08-17 — Gameplay → *Camera Frame*,
world-fixed / player-relative, written onto the view component from
`GameplaySettings::camera_reference_frame`. ⭐ the input pairing needed no
second setting: a player-relative view collapses every `InputFrameMode` onto
body-relative as an identity. ⛔ **do not continue it as a standalone campaign.**
C5 — camera policy read off the view index — is N-VIEW work and belongs to D116;
the feel questions (shake units, acceptance customers) are filed in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §26.

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
settles for `attack_side`, then `attack`, then `slash`, then `idle`) — so an
unauthored fighter frame costs a move its picture and never its gameplay.

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

⭐ **OFFER / CONSUME beats OVERRIDE.** A character DECLARES a pile of properties;
a ruleset READS the subset it cares about — nobody overrides anything, weight
becomes something George SAYS about himself that a fighting ruleset reads and a
platformer ignores.
⭐ **the refinement that holds it up under balance pressure: the CHARACTER
authors the PROPERTY, the RULESET owns the FUNCTION from property to effect.**
George says he is heavy; the smash ruleset decides what heaviness DOES. Balancing
is tuning the function and choosing the cast — never rewriting a character. The
cast-relative reference frame (George's 1.35 against v3's 1.0) dissolves once the
property is stated against a FIXED reference body rather than whoever is on the
grid today.
⚠ **the genuinely fuzzy residue is hitbox/hurtbox GEOMETRY**, where one body
needs different answers per genre. Offer/consume covers it — author both, each
ruleset reads what it needs — but it is unproven; grabs/techs/hurtboxes are not
authored yet. One data point is not a shape.

⛔ **WHAT IS OWED WHILE THIS IS DEFERRED — restitch cost, and only that.** Jon's
condition: *"as long as the seam isn't too difficult to maintain or hard to
restitch."* The invariant to hold: **every game-adjusts-a-character edit goes
through ONE NAMED COMPOSITION SEAM, never a reach-in.**

| adjustment | form today | restitch cost |
|---|---|---|
| abilities | `effective_abilities` — stated once | cheap |
| body | `MatchRules::body_over` — stated once | cheap |
| `knockback_weight` | `install_smash_content` MUTATES `definition.vitals` in a loop | ⛔ **a reach-in** |

▢ normalize the third onto the same seam as the other two. No direction is
implied and no decision is taken by doing so — it is the shape that makes either
answer a one-edit change later.

⭐⭐ **THE SHARPENED VERSION — "DEFER THE UNIVERSAL CHOICE, BUT NOT THE
BOUNDARY."** (GPT's framing, Jon endorsed: *"I agree with them"*, 2026-08-16.)
Instruction, verbatim: *"Keep the current D146 work moving. **Do not design the
final universal character/game composition model from one weight customer.** But
**eliminate the registration-time reach-in.** Put Smash's interpretation of
character-authored data behind **one pure named preparation/projection seam.**
Treat character authoring and ruleset specificity as **orthogonal**: a future
`SmashFighterFacet` can be **authored with the character while being owned
semantically by the Smash capability**. Shared properties should only migrate
into the common character/body schema **after multiple real consumers prove they
are actually shared.**"*

⭐ **ORTHOGONALITY**: WHERE a fact is authored (with the character, in its own
crate) and WHO OWNS ITS MEANING (the ruleset whose vocabulary it speaks) can
differ, and normally should.

⚠ **this is not speculative — D144 already built one.** Mary-O's smash table is
in HER crate (`game/ambition_demo_mary_o/src/smash_moveset.rs`) and speaks
SMASH's vocabulary, unreachable at home because her ability row omits `attack`.
`MatchAbilities` / `MatchBody` are the ruleset-owned half of the same idea.

▢ **THE ARCHITECTURAL HYPOTHESIS, RECORDED FOR LATER — do NOT build it yet:**

```text
CharacterSpec is NOT "every mechanical truth about this person".

CharacterSpec/package
    = a FEDERATED COLLECTION OF AUTHORED FACETS.

A game/ruleset CONSUMES the facets it understands
    to prepare a body/role for that experience.
```

A character does not carry a union of every game's needs, and no game overrides
an author fact — each ruleset reads the facets it speaks and ignores the rest. An
unauthored facet is a NAMED GAP (see the tumble ruling above), not a silent
default.

⛔ **the migration rule, and it is a brake**: a property moves into the COMMON
character/body schema only after MULTIPLE REAL CONSUMERS prove it is shared. One
customer is a facet. ⚠ so `knockback_weight` gets a seam, NOT a schema.

**THREE ITEMS, in the order they should be done. Everything below is MEASURED,
not assumed — the reading was done 2026-08-16 before any of it was written.**

**1 ✔ DASH OUT OF THE SMASH KIT — CLOSED 2026-08-16 (`6db8cab2c`, `a7b5ab681`,
`f4210ba19`, `c0208b21b`).** Dash removed from the smash ability kit; no
compensating number needed —
`removing_the_dash_from_a_dodging_kit_changes_no_reach` measured identical
recovery with the dash bit on and off, because dodge already outranked dash on
the shared press. ⛔ the kernel filled the shared dodge/dash buffer only for
`abilities.dash`, so deleting `dash: true` alone would have deleted the DODGE
from all fourteen fighters in silence; `apply_intent` now gates on `dash ||
dodge` and the field is `buffer_burst`. The CPU's dash-to-close became
`SpecificAction::Sprint`/`sprint_to_close`/`smash_sprint_to_close` (D146-1),
since closing distance is not a capability.

**1b ✔ THE STAGE LEVELS ABILITIES AND NOW SUPPLIES THE BODY THEY RUN ON —
CLOSED 2026-08-16 (`9817eb949`, `205e52a5e`, `6a74247b5`, `441a0b7cc`).** Found
while doing item 1: only three characters authored `movement_tuning`, so an
airborne burst press on everybody else resolved to nothing once `dash` left the
kit — measured worse than assumed (twelve of fourteen, not eleven, since two ids
are stand-ins the host drops for the real lineage). `MatchBody` (core, beside
`MatchAbilities`) is the six numbers a MODE owns — `slash_recoil`,
`jump_squat_time`, the air-dodge window/speed/endlag, the tumble floor —
composed via `MatchRules::body_over` onto whatever body a fighter brought.
⛔⛔ a whole `MovementTuning` spreading `..DEFAULT_TUNING` was tried first and
was wrong: it states every field whether or not its author had an opinion —
`the_puppy_slug_forced_onto_the_stage_keeps_the_body_it_authored` caught the
crawler's authored 80 px/s becoming the engine's 270. Same trap `MatchAbilities`
already names on the grant side.
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
criteria: *"Shield input -> can hold/release shield. Special input -> activates
authored special behavior. One cannot accidentally masquerade as the other."*
Fixed: `resolve_control_slots`'s `ControlSlot::Shield` arm (renamed from
`QuickAction`; semantic id `quick_action`→`shield`, preset key likewise) now
mirrors Attack — absent slot strips the verb, held item keeps it, technique
routes it — instead of `gate_worn_player_control` clearing `shield_held` unless
`ActionSet.special == Special("bubble_shield")`, which lost the guard on any
persona whose special wasn't literally that string.
`ControlSettings::migrate_renamed_actions` rewrites stored `"QuickAction"`
overrides. The CPU's `tick_smash` now commits `SpecificAction::Shield` instead
of writing `shield_held` by hand.
⛔ the probe that claimed a human smash fighter could never shield was itself
wrong — a smash seat carries no `PlayerEntity` (`realize_seat` uses
`EnemyActorBundle`, `NoInitialBody`), so `gate_worn_player_control`'s
`With<PlayerEntity>` filter never ran on a fighter; a gate's query filter IS the
blast radius.
⚠ the shipped smash CPU is `template: Fighter`, not the smash brain, so its
guard comes from `MovementVerb::Shield`.
Evidence: `a_smash_fighters_shield_input_raises_and_lowers_their_guard`,
`pressing_special_does_not_raise_a_guard_on_a_fighter_whose_special_is_not_one`,
`holding_shield_raises_a_guard_and_fires_no_authored_move`,
`a_cpu_fighter_raises_a_guard_without_pressing_a_physical_button` (all
`smash_in_the_host`), `the_shield_verb_follows_the_ability_not_the_special`,
`a_held_item_keeps_the_shield_verb_alive_without_the_ability`.
▢ **NOW UNBLOCKED, and recorded rather than done: dodging comes off the SHIELD
button.** In the genre shield+direction is a roll, shield on the spot is a spot
dodge, and shield in the air is an air dodge — none of them a separate burst
button, which is where they live here. It needed Shield to be a real action
first; it is one now.

**3 ✔ THE PAD LAYOUT, AS A PROFILE RATHER THAN A DEFAULT — CLOSED 2026-08-16.**
Jon's layout, live on a pad: A=normal, X=special, B=jump, Y=grab (blank, see
below), LT=shield — a DECLARATION, never a preset edit; A=Jump stays Ambition's
default and the release gives the pad back when the experience leaves.
`BindingLayout` (`ambition_input/src/layout.rs`) is a third layer in
`BindingRecipe::build`: base preset, then the GAME's layout, then the USER's
overrides — pinned by `a_user_remap_beats_the_modes_layout`. It is keyed by
BUTTON rather than `Vec<BindingOverride>`, so no button fires two actions and a
displaced action loses its pad binding but keeps its key (Blink, Projectile,
Utility, Modifier lose pad buttons here; menu actions are exempt via
`is_menu_only`). Both shoulder buttons (`LeftTrigger`/`LeftTrigger2`) shield.
⛔⛔ Y is a declared blank pending Grab — one line in `SMASH_PAD` once that
vocabulary item exists. Bindings are carried by
`apply_active_binding_layout_to_recipes` onto every `InputParticipant`, not just
the primary (`realize_seat` spawns bodies with no `InputMap`; the map lives on
the participant entity), and the settings→recipe sync carries it forward.
Evidence (`smash_in_the_host`): `on_the_smash_pad_x_fires_the_fighters_authored_special`,
`on_the_smash_pad_the_left_trigger_raises_a_real_guard`,
`on_the_smash_pad_b_jumps_and_a_attacks`,
`quitting_a_smash_match_gives_the_pad_back`; plus `ambition_input::layout`'s
`every_button_the_smash_layout_claims_drives_exactly_one_verb`,
`the_actions_smash_displaces_lose_the_pad_and_keep_the_keyboard`,
`a_layout_rearranges_gameplay_and_leaves_the_menu_alone`,
`installing_the_smash_layout_does_not_move_the_generic_preset`.
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
until today referenced by no ledger by its own instruction. Jon promoted it:
*"a character pack might be a good way to author character height in some shared
world units so we can get a sense of the scale at which characters should
render."*

⭐⭐ **THE SLICE IS CHOSEN SO THE PACKAGE FORMAT IS DEFINED BY A REAL CUSTOMER**
rather than argued up front. The customer is three of Jon's own reports that are
one defect.

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
   effectively use, so this is mostly declaring what is already implied. ⚠ a
   quality tier scales the ART, never the declared height.
2. **height is a CONTRACT**: art scales to it, so the cast is consistent by
   construction and a badly-framed sheet cannot make a character huge. A tight
   tolerance **WARNS** when the scale factor drifts. ⛔⛔ warns, does not refuse —
   that word is Jon's, and it is what separates this from a gate. ⚠ pick the
   tolerance from the measured population and state it; do not invent a round
   one.
3. **landmarks are OPTIONAL SLOTS** — head/feet/hands/sockets authored where
   useful, and every consumer must work without them. ⛔ never make one required
   to satisfy a consumer. ⚠ we may eventually have skeletons available in game,
   and a skeleton subsumes hand-authored landmarks.
4. **promotion did not schedule the other eight milestones.** A slice becomes
   work when something asks for it.

⛔⛔ **BOSSES AND GIANT BODIES ARE IN SCOPE — ruled 2026-08-17, and it roughly
DOUBLES this slice.** A boss is a character that happens to be large: same units,
same contract, and a multi-part body declares the height of the whole SILHOUETTE.
The boss sheet path computes its own render height today
(`collision.max_axis * collision_scale`, authored at 4.5 / 1.8 / 1.6 / 1.25) and
must derive from the declared height instead. Taken deliberately over an
ordinary-cast-first slice: an exemption meant to be temporary is exactly the kind
that becomes permanent — **an exemption list is a TODO list.**

⛔ **`collision_scale` stops being a SIZE knob and is NOT deleted in this slice.**
Its own doc says what it actually is — *"a multiplier on the actor's collision
AABB… authored per-character to compensate for the fraction of each frame the
character art occupies after auto-crop"* — i.e. a PADDING compensation being used
as a size control. Height replaces the second job, not the first.

⚠ **the known trap, measured on the earlier attempt**: sizing the quad from the
body bbox WITHOUT also cropping the drawn region was tried and reverted because
it stretches the art badly. It needs **four** coupled sites, not the three the
design doc names — there are two render-size publishers, and fixing one leaves
both of the characters Jon complained about untouched. Find both before editing
either.

⇒ **acceptance is Jon's three reports, not a number**: the snake and AI slop,
Sanic in his own game, and the cove pirates against the robot. If declaring
heights does not settle them, the quad-from-bbox route comes back with evidence
rather than with an argument.

✔ **SLICE 1 LANDED 2026-08-18 — the vocabulary exists and its first customer
states its height.** `Vitals::canonical_height` (world pixels, 16 to a tile) plus
`world_per_pixel_for_height`, and the robot lineage now DECLARES 48 rather than
spelling the division. No behaviour changed: 48 / body_px.y is exactly what it
computed before.

✔ **MARY-O'S ART NOW SCALES FROM A RIG, LANDED 2026-08-18** (renderer submodule
`2813531`). Her parts hung off the GROWN form's absolute offsets, so
re-proportioning the short form to one brick broke them one at a time.
`FormRig` states where parts belong as fractions of the form's own authored
size, solved from the approved grown form, which stays byte-identical through
the change. ⛔⛔ the correctness argument is WHICH INPUT OWNS WHAT: the pose owns
placement (crouch, lean, bob), the form owns the proportions the fractions
multiply — deriving the hip from the form alone silently dropped crouch, and
scaling x by the crouch-widened width moved every skid frame; neither was
visible without differencing renders.

✔✔ **MARY-O IS ONE BRICK NOW** — the rescale that was "blocked" was a UNIT
CONVERSION. `SMALL_FORM_HEIGHT` is `T` (32 world units, one tile) instead of 48,
so she stands one block small and two grown, which is Jon's ruling.

⛔⛔ **THE BLOCKER WAS AN ARITHMETIC ERROR IN THIS LEDGER, AND IT COST A REVERT.**
An earlier attempt read Jon's *"16 units"* as this demo's world unit, set the
constant to 16, watched the flagship vault break, and concluded a level-wide
rescale was owed. But `defaultGridSize: 16` is the LDtk AUTHORING grid; the
generated 1-1 those vault measurements live in is authored on `T = 32` world
units per tile — one block here is 32, and 48 was never "three blocks", it was
1.5 tiles. ⇒ **the level needed no re-authoring at all** —
`a_pipe_you_enter_always_has_a_pipe_you_come_out_of` passes at the new size, and
so does the whole workspace. The *"60 units of reach"* this row called a second
blocker was measured in the same mistaken unit. What DID have to land first was
the art: at 1.40:1 no single scale reaches 1:2 without widening her 1.43x, which
`her_forms_are_all_the_same_width` refuses on a gameplay rule — the rig work made
the ratio exactly 2.0.

⚠ **two things this forced are WAITING ON JON**, recorded as §14 in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md): the shared
collision width went 64 → 56 px (one width for every form, decided by the
narrowest form), and her one-brick box has 6 px of headroom above her hat
because the box top comes from the height contract rather than the art.

⭐⭐ **AND THE UNIT WAS ALREADY THERE, WHICH MAKES THE RULING CHEAPER THAN IT
LOOKED.** `DEFAULT_PLAYER_BODY_HEIGHT` is 48 world pixels — exactly three tiles
at `defaultGridSize: 16` — and the field's own doc calls them *"world pixels"*.
So Jon's *"one world unit = one base-grid pixel"* DECLARES what the engine
already used; nothing converts.

⛔⛔ **WHAT WAS MISSING WAS NOT A UNIT BUT AN AUTHORED NUMBER — three characters
were each deriving the same scale by hand, and not even on one axis:**

```text
player_robot_lineage.rs:203   world_per_pixel = DEFAULT_PLAYER_BODY_HEIGHT / px.y   ← height
ai_slop.rs:177                world_per_pixel = AI_SLOP_BODY_WIDTH      / px.x      ← WIDTH
snake.rs:412                  snake_world_per_pixel(), an opaque helper
```

⇒ the slop's HEIGHT is whatever its art's aspect ratio produces, because nobody
ever stated it. Same defect as `collision_scale` one layer up.

⛔⛔ Jon's own measurement redirected the next slice: *"their collision bodies
are the right size in world units now (snake 1.00x Mary-O's width, slop 1.09x),
and what is left is that the drawn quad is 2.46x the body inside it."* The
collision derivation was already correct for the characters he complained about
— the QUAD was what was wrong.
⚠ read `spawn_actors.rs:733`'s comment before touching it: the shared
`ActorRenderSize` exists precisely so a hostile flip cannot re-apply
`collision_scale` and balloon the sprite a second time.

⛔⛔ **THE OBVIOUS FIX — moving those characters onto the "already correct" road
— IS WRONG, MEASURED AND PHOTOGRAPHED 2026-08-18.** It cannot close the 2.46x,
because the snake is ALREADY on that road (`SpritePosedBody`) and still measures
2.46x; adding it to AI Slop left its drawn size unchanged (48x55 → 48x54 px).
⭐⭐ **the number is a FRAME-vs-BODY fact**, already measured, named and
ratcheted by `enemy_quad_matches_its_box` (`QUAD_OVERHANG_LIMIT = 2.47`) — where
Jon's *"2.46x"* comes from. The snake's sheet publishes a body of **117 x 52 px
inside a 128 x 128 frame**, and `PosedBodyGeometry::render` draws the whole
sheet frame: the quad is SQUARE while the animal is 2.25:1, at every scale, on
either road. ⇒ no amount of re-wiring who publishes the size changes it; what
changes it is drawing the body's SUB-RECT, the art-crop already tried-and-
reverted for stretching the art.

⭐⭐ **THE TRIM MECHANISM ALREADY EXISTS AND IS LIVE — measured 2026-08-19,
moving this row from "build the trim" to "REGENERATE 61 SHEETS".**
`trimmed_render` + `FrameTrim` are built, exported and consumed by
`character/animator.rs`; a sheet opts in by publishing per-frame rects with a
non-zero `off`, and `SheetRow::is_trimmed` is False for legacy uniform sheets,
which keep the cheap fixed-anchor path.

```text
TRIMMED sheets    133   quad is the frame's own rect
UNTRIMMED sheets   63   quad is the WHOLE frame — the legacy path
```

⛔⛔ the snake is in the untrimmed 63 and the AI Slop is NOT, which is the whole
of why they measured differently: `snakes_on_a_cartesian_plane` publishes ZERO
per-frame rects, while `ai_slop` publishes 44, every one with a non-zero offset.
Nothing needs building.

The 61 untrimmed sheets that publish a body bbox, worst frame-vs-body area
first — the regeneration queue:

```text
 10.8x  super_mary_o_coin           frame  96x96   body  23x37
  6.5x  mary_o_v2                   frame 160x192  body  56x84
  5.3x  super_mary_o_milk_carton    frame  96x96   body  33x53
  4.0x  sandbag                     frame 128x128  body  48x85
  3.7x  carl_stargan                frame 155x157  body  58x114
  3.3x  snakes_on_a_cartesian_plane frame 160x128  body 123x50
  3.3x  mary_o_v2_tall / _fire      frame 160x192  body  56x168
```

⚠ **the sentence after the measurement**: a frame-sized quad is extra
TRANSPARENT margin, invisible on its own — `collision` comes from
`body_pixel_bbox` and is unaffected. It becomes visible exactly when something
SCALES the quad, which is what the height contract (this row's own slice 2)
does: scale the frame to a declared height and the body inside lands at
`1/overhang` of the size intended. ⇒ the untrimmed 63 are a prerequisite for the
height contract, not an independent defect — Mary-O v2 at 6.5x is first in line,
since she is the character the contract is being proven on.

✔✔ **THAT OPEN THREAD IS EXPLAINED — 2026-08-18: the slop's sizing WRITES A
MIRROR, NOT THE AUTHORITY.** Two single-variable poisons of the same body had
disagreed:

```text
AI_SLOP_BODY_WIDTH 28 -> 60      drawn slop 48x55 -> 48x55 px   ZERO change
body.half_size     -> 4.0        drawn slop 48x54 -> 18x21 px   follows it
```

Measured on the live entity rather than the sizing function:

```text
ai_slop_half_size()        28.0 x 18.2     what the constant derives
CenteredAabb @ tick 2      28.0 x 18.2     tag_mary_o_ai_slop's write LANDS
kin.size / BodyBaseSize    73.87 x 48.00   the AUTHORITY — never written
CenteredAabb @ tick 400    73.87 x 48.00   re-derived from the authority
```

`tag_mary_o_ai_slop` does `body.half_size = ai_slop_half_size()` on
`CenteredAabb`, a DERIVED MIRROR; `reset.rs` does `aabb.half_size = em.kin.size *
0.5`, so the spawn size comes back — the constant reaches the mirror for two
ticks and never reaches the body. ⛔⛔ the guard beside it is structurally blind
to this: `the_ai_slops_box_has_the_shape_its_sheet_publishes` asserts against
`ai_slop_half_size()` — the FUNCTION — proving the arithmetic but never the
wiring. Same lesson the human-grab defect taught one layer up: *a test that
starts downstream of the wiring cannot see the wiring.*

▢ **the fix is one line and it is NOT TAKEN HERE, on this row's own rule.**
Writing the authority (`kin.size` + `BodyBaseSize`) instead of the mirror makes
every slop 28 x 18.2 rather than 73.9 x 48 — a **2.64x shrink** in a level Jon
plays. This row already says *"how big a slop should be is a taste call for
whoever is looking at the running game"*, so the size is his and the defect is
that the intended value never took effect. ⚠ `SpritePosedBody` is NOT the
overwriter — checked and absent on all twelve slops, so the sprite road is not
involved and the 73.87 x 48.00 comes from the spawn.

⛔⛔ **AND THE ONE-BRICK RESCALE HALVED THE SNAKE, WITH NOTHING SAYING SO.**
`snake_body_width()` derives from `mary_o_body_width()`, so when she became one
brick the snake followed her down: `world_per_pixel` 0.35 -> **0.182**,
collision 41 x 18 -> **21.3 x 9.5**, which is **0.30 tiles tall**. ⭐ the ratchet
beside it could not notice — it pins the quad/body RATIO, which is
scale-invariant, so it read 2.46x before and after: a value derived from another
character moves when that character does, and a ratio test is structurally blind
to it. The two docs that quoted the old sizes are corrected; whether a
third-of-a-tile snake still reads as an enemy is a look-at-it call, and the
constant to change, if any, is HERS, not the snake's.

- ✔ **D164 — CLOSED 2026-08-18.** Two top-level plans looked stranded; the audit
  had enumerated the wrong three files.
  [`sprite-residency-and-live-quality.md`](sprite-residency-and-live-quality.md)
  (steps 2–5 open) and
  [`frontend-audio-is-per-experience.md`](frontend-audio-is-per-experience.md)
  (one open step) are both listed with ▢ entries in
  [`tracks.md`](tracks.md), since `594a548bf` — the row's evidence ("referenced
  by neither `queue.md` nor `roadmap.md` nor the README") was true but did not
  support the conclusion, because `tracks.md` is the standing backlog and was the
  one index not checked. ⛔ an ABSENCE claim is only as strong as the set of
  places you looked, and a hand-written set of places is a guess.

- ✔ **D163 — CLOSED 2026-08-18. The validator's errors are 0 and its loudest
  warning no longer flags a designed relationship. (opened 2026-08-17)**

```text
                    was                              now
error:              30, ALL false positives          0
spawn_overlap       8, every one a rider on a mount  0 (mounts exempt; real overlaps still fire)
missing_level_wall  portal_lab false + genre pits    5, all genre pits (a bottomless pit IS the design)
editor.shape        8 entities unplaceable in 2      6, all `SurfaceRamp`, deliberately deferred
```

⛔ the one thing left is a PRODUCT CALL, not a defect: `SurfaceRamp` has a
converter, a winding oracle and 0 placements in any world, so whether to invite
authors into an unused capability is Jon's.
⚠ the `sanic_sandbox` off-grid origin is AUTHORED (`world_y: 3000`, spec and
level agree) — not drift.
▢ moving it means editing spec and level together, for one level in the whole
project.
▢ who owns a level's POSITION, the area spec or `world auto-layout` (§16 in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md)).

⛔⛔ the validator's own errors were 100% noise and nearly cost the
shark-riding pirates: a "two pirate-sky rooms ship seven DUPLICATED enemy
spawns" reading was about to run `ambition-ldtk entity delete`, but a rider and
its mount are AUTHORED at the same pixel (the raider's `mounted_on` names the
shark's entity iid) — deleting either would have destroyed content Jon already
once reported missing. ⛔ compare FIELDS before calling two entities duplicates.

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

▢ **the two that survived scrutiny:**
1. ✔ **`portal_lab`'s bottom edge was a FALSE POSITIVE.** `missing_level_wall` probes only the outermost cell row, while portal_lab's full-width floor sits five rows above the boundary. Fixed by asking whether a floor blocks a fall wherever it is. ⚠ only the BOTTOM gets this — the same idea on left/right is much noisier (46 open sides vs 6), because a corridor's side wall legitimately has a doorway gap.
2. **`SurfaceRamp` has no editor definition**, so a supported engine entity cannot be placed by an author. ⛔⛔ that reasoning covers only `SurfaceRamp` — the `defs.entities` warning names EIGHT entities, and the other seven (`GravityZone`, `GroundItem`, `Portal`, `PortalGunSpawn`, `ShrineSpawn`, `SurfaceChain`, `SurfaceLoop`) are placed and defined in four of six worlds but missing from `intro.ldtk` and `you_have_to_cut_the_rope.ldtk` — an author in the flagship world could not place a `Portal` used next door in `sandbox`.
   ✔ **RECONCILED 2026-08-18.** The seven were copied into `intro` and `you_have_to_cut_the_rope` via `def upsert-entity` from `sandbox`'s spec. All six worlds now carry 33+ defs and differ only in `SurfaceRamp`; every world validates with 0 errors.
   ⚠ **`SurfaceRamp` stays out on purpose** — 0 placements in any world, a deliberate product call.

✔ **`spawn_overlap` KNOWS ABOUT MOUNTS, 2026-08-18.** A pair joined by `mounted_on` is exempt, since position-identical is what a mount is and only the fields distinguish a relationship from a duplicate. Measured: sandbox 5→0, intro 3→0. Two unrelated spawns at one pixel still warn.

⛔⛔ **`level diff-specs` was reading nothing — found 2026-08-18.** It loaded only `.yaml`, and every area spec is `.ron`, so it reported success by finding nothing to check. ✔ fixed: the loader reads RON through the tool's own `ron_parse`, and `--all` globs `.ron`/`.json` too. ⚠ turning it on reveals real drift (52 of 54 specs differ) and it is NOT wired into CI yet.

▢ **the question underneath it is who OWNS a level's position** — the spec, or `world auto-layout` which arranges levels by the LoadingZone graph. The tool's own message says "live LDtk wins," suggesting the specs stopped being authoritative and nobody re-recorded them. ⛔ do not bulk-rewrite the 52 specs to silence it before that is answered. (`sanic_sandbox`'s off-grid Y is NOT drift — it's authored in `specs/sanic_sandbox_area.ron` and matches the live level.)

✔ **AND THE 30 ERRORS ARE 0, 2026-08-18 — RESOLVED, NOT SUPPRESSED.** Both causes were the validator being handed less than the runtime has: 4 cross-world targets (secondary worlds now default to sibling worlds beside the file, `--no-sibling-worlds` opts out) and 26 unknown entities (`mary_o.entities.json` sits beside the symlink, not the real file — the sidecar search now checks both).
⛔⛔ **the default belongs in the LIBRARY, not a CLI parser** — it was in the parser first and `repair` walked past it, so `entity set-field` still failed on errors it was meant to clear. Every entry point (`validate`, `repair`, `repair_and_validate`) now reaches one line.

- ▢ **D162 — REOPENED 2026-08-18: the SheetRegistry dismissal rested on a
  reporter that never ran, and running it finds THREE real ones.** (was CLOSED
  2026-08-17, four standing boot warnings triaged)

⛔⛔ **"no character id collides" was measured from a silence that meant "I did
not run."** `report_shadowed_character_sheets` is a `Startup` system;
`init_sheet_registry` is ALSO `Startup`, and Startup is unordered, so it ran
with `Res<SheetRegistry>` absent and printed nothing on every route — while the
registry logged 39 shadowed targets in the same boot. ⇒ moved to `PostStartup`.
⚠ "the catalog knows this name" was also the wrong filter: of the 39,
`robot`(15)/`goblin`(8)/`sandbag`(1) ARE catalog ids but they are shared RIG
adapters, not collisions — that filter would have reported 24 legitimate rig
shares as defects.

Asked against `ShadowedTarget::loser_image` instead, **three survive and are
real:**

```text
robot     robot_spritesheet.png      256x256  LOSES to robot_archivist        230x256
goblin    goblin_spritesheet.png     239x253  LOSES to goblin_brute_hammer    232x256
sandbag   sandbag_spritesheet.png    128x128  LOSES to sandbag_armored_review 256x256
```

⛔⛔ **the harm is not demonstrated the obvious way.** The character geometry
road (`record_for_target` → `record_index()`) keys by FILENAME ROOT and cannot
collide. What collides is the target-keyed `SheetRegistry` RESOURCE, whose four
consumers (`bosses/sync.rs`, `slash_visuals.rs`, `shrine_visuals.rs`,
`projectile_visuals.rs`) do not appear to resolve those three names — so three
real collisions exist in a registry whose current readers don't hit them.

⭐⭐ **the ambiguity is structural — measured across all 196 baked sheets:** 52
sheets have a `file_root` differing from their `target` (authored against a rig
adapter); 5 targets are each claimed by more than one file, 48 files between
them (`robot` 18 · `toon` 16 · `goblin` 9 · `sandbag` 3 · `ninja` 2). The
target-keyed registry cannot answer "give me sheet X" for any of those 48 —
which wins is load order. `record_index()` already keys by file root (196
unique keys) via `from_baked_table_by_file_root`.

▢ **what is left: (a) retire the stale manifest in each of the three pairs —
a content call the registry cannot make — and (b) DECIDE THE KEYING**, since
for the 148 sheets where root == target the two are identical and switching
would make this whole class impossible rather than reportable. ⛔ not taken
unilaterally — it changes what a shared engine resource returns for 48 files.
Asked as **§19** in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md),
with both options and their blast radius.
⚠ the sandbag pair is loudest if it resolves wrong: a 128px sheet cropped on a
256px grid.

✔ **a fifth boot warning found while measuring the above, fixed 2026-08-18.**
`mary_o::quasar: overlay not attached` printed on the FIRST frame of an
ordinary texture decode instead of only once it PERSISTED. ⇒ counted per
candidate, reported once past `QUASAR_ATTACH_GRACE_FRAMES` (60). Poison-verified:
unmodified a 150-frame boot now prints zero warnings; with attachment made
impossible it prints exactly one.

✔ **loading zone "did not fire" — CLOSED (`ad82531b7`).** WARN only on a
PRESSED door, DEBUG otherwise; verified 1→0 warnings, door still transitions
under `--press f`.
⛔⛔ **the first attempt at this fix was wrong in a way worth keeping**: it
filtered the message away whenever no press was buffered, which would have
silenced the instrument in the exact scenario that justified it — a broken
binding is exactly the case where the player pressed and `wants_interact`
still reads false. ⇒ a value a diagnostic REPORTS is a bad value to gate it on.

⭐⭐ **the flagship boot inventory, measured 2026-08-18 on `--route
ambition_gameplay` — every line now has a verdict:**

```text
sanic_sandbox off-grid Y     ▢ D163 — blocked on §16 (who owns a level's position)
GgrsSchedule redundant edge  ✔ WON'T-FIX — both memberships individually correct
SheetRegistry robot/goblin/  ✔ CORRECT and newly VISIBLE — the three above; keying is §19
  sandbag
room has 38 neighbours       ✔ LEAVE IT — warn_once!, names its constant, and its author
                                argued the case: "a cap that quietly drops work reads as
                                everything is prefetched"
npc_kernel_guide             ▢ NEW, and worth someone's attention
```

▢ **`NpcSpawn-0017` names `npc_kernel_guide`, which the composition has not
registered**, so it falls back to its catalog row's body with a borrowed kit —
correct only for a BORROWED character in a partial composition, and
`ambition_gameplay` is not one. The character is authored into
`hall_of_characters.ldtk` and `sandbox.ldtk`, has its own spritesheet, and the
intro road resolves it by id. ⇒ a member of the room that exists to show the
cast off is running on somebody else's kit. ⚠ whether it should get an
authored `CharacterDefinition` is a content call; check the D56 note first —
the Kernel Guide "leaves it blank so kernel→goblin keeps its visual gag," in
case the borrow is the joke.

- ✔ **D161 — CLOSED 2026-08-18. No loading zone prints an authoring id any
  more: 130 → 0, and a per-world ratchet in CI keeps it there.** (opened
  2026-08-17, found by CAPTURE of `intro_wake_room` — the flagship's opening
  room)

`spawn_loading_zone` in `crates/ambition_render/src/rendering/world.rs` renders
`zone.name` unconditionally for non-Door zones (Door zones get a
proximity-gated nameplate instead), and a zone's name is a level-authoring
identifier, not prose. Measured: of 151 named loading zones, 130 (86%) were
snake_case ids, 19 of those not gated by Door and so always shown.
⛔ CORRECTED 2026-08-17 — first published doubled (302/260/38) by counting
`game/ambition_content/assets/worlds/` and `game/ambition_map_assets/*/worlds/`
as separate, when they're the same worlds.

✔✔ **AUTHORED 2026-08-18 — 130 → 0.** Every loading zone carries
player-readable prose, each following its own level's authoring convention;
destinations came from the zone's own `target_room`, never from prettifying
the id (`wake_to_raid` has no good rendering that way).
⛔⛔ dismissing the bulk as "a developer sandbox where a diagnostic id is
defensible" was WRONG — `sandbox.ldtk`'s `central_hub_complex` is the world
manifest's `entry_room`, so 17 of the ids were in the game's first room.

✔ **the ratchet is `scripts/check_zone_name_ratchet.py`** (baseline
`dev/zone_name_ratchet_baseline.json`, now an empty map), per world, fails if
it observes no zones at all. Runs in CI. ⛔⛔ dedupes by real path — every
world under `game/*/assets/worlds/` is a symlink into
`game/ambition_map_assets/`, the same doubling trap as above. ⚠ `None` ≠ `{}`
in its baseline loader — an empty map is the goal state, not "never recorded."

⚠ the room's `→ corridor` label that looked correct in the same frame is a
`DebugLabel`, not a zone name — so the opening room also has duplicated
signage. ▢ whether a non-Door zone should draw an unconditional world label at
all (24 named zones are `EdgeExit` and always draw) is asked as §17 in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), alongside:
12 rooms carry both authored signage and always-on zone labels, and
`gate_stack_lower` has fourteen `DebugLabel`s doing player-facing work.

⚠ a separate genuine dev-note leak, found in the same sweep: `central_hub_main`
carried a sign whose text described the LDtk authoring artifact rather than the
fiction — of 134 sign texts, only this one failed that test (42 others read as
the game's intentional `//`-prefixed lab-AI house style).
✔✔ **REPLACED 2026-08-17 BY MAINTAINER RULING.** Jon, verbatim: *"replace it,
don't delete it. The hub benefits from an orientation sign; only the
authoring-language content is wrong."* Landed via `ambition_ldtk_tools entity
set-field` (not a direct JSON edit, which he named as part of the decision);
EntityRef counts checked unchanged before/after.

- ✔ **D157 — CLOSED 2026-08-16. Mary-O had her whole smash moveset in her own
  platformer: `combat_actions` derived attack slots from the MOVESET and
  `ActionSet` and never read `AbilitySet`, so `abilities: Some([RunJump])`
  bought nothing — twenty-three distinct swings reachable.** Fixed:
  `combat_actions` takes the `AbilitySet` and ceilings the melee family with
  `abilities.attack`. ⚠ Projectile is deliberately NOT under it — a ranged
  verb is always an explicit grant.
  ⛔⛔ a test caught this and was argued away: `…peaceful_kit…` asserted
  `moveset_len == 0`, went red at 17, and was rewritten to agree with the 17
  on an intent nothing implemented. Guarded by `mary_o_at_home_can_only_run_and_jump`,
  `the_run_button_throws_a_spark_only_while_she_wears_the_lantern`,
  `the_demo_body_cannot_trigger_a_single_move_from_its_own_smash_table`.

- ✔ **D156 — CLOSED 2026-08-16. The Patent Clerk faced backwards: facing was
  authored three times and nothing in Rust read any of them** —
  `gravity_aware_flip_x` was `facing < 0.0` with no per-character term,
  assuming all ~800 baked sheets face +x.
  ⛔⛔ it was a FORK, not a missing feature — `animate_bosses` already XORed
  this term. Landed `SheetRecord::authored_faces_left`, lifted
  `data-rig-facing` onto `CharacterSpec` (`8c30de613`/`37ac258b6`/`fd4320071`;
  renderer `fac948b`→`9b445c5`).
  ⚠ `SpritePackCatalog::to_sheet_record` synthesizes from atlas rects and
  cannot know facing, so the pack now inherits the base manifest's. ⚠
  `facing: str = "west"` is the DEFAULT, so "the rig says west" can mean
  nobody set it — guarded by `every_baked_sheet_is_drawn_pointing_where_its_body_faces`,
  pinned to exactly eight declaring manifests.
  ▢ still open, small: the portrait tier declares no facing and was never
  checked; two pre-existing rig-validator failures at HEAD (Carl's
  paint-slice order, Noether's `head_base`/`head_features` naming) — only
  `validate` is red, `build` is fine.

- ✔ **D158 — CLOSED 2026-08-17, then SUBSUMED BY D159. Two taunting CPUs
  printed through each other because `stack_offset` was measured from each
  speaker's own head, and floor-to-air is one stack step** — the ordinary
  geometry for a platform fighter, not a corner. Its bespoke stacking
  mechanism is DELETED: a bubble is now a `WorldLabel` in the one ranked
  placement pass (D159).
  ⛔ do not reintroduce a second system that places bubbles — two placement
  passes that cannot see each other is how this bug happened, and each could
  truthfully report "no overlaps found."

- ✔ **D160 — CLOSED 2026-08-17 BY MAINTAINER RULING: the omission is
  deliberate. The cheap unit tier is a REQUIRED PRE-PUSH check, not a
  per-turn gate — two tiers on purpose.** Jon: *"keep the per-turn gate small
  … The workspace lib suite remains a required pre-push/finalization check …
  'Gate' should continue to mean an executable gate."* Three tiers:
  `scripts/gate_suite.py` per turn (stays cheap); `cargo test --workspace
  --lib` pre-push (required is not gated); feature-gated suites when
  touching the subsystem.
  ⛔⛔ **DO NOT ADD `--workspace --lib` TO `gate_suite.py`.**
  ⛔⛔ this row was once closed on a false premise — it claimed the sweep was
  "added to the stated gate," but what landed was a paragraph in
  `AGENTS.md`. Two commands run in one turn is not one command invoking the
  other; "in the gate" means a line in `gate_suite.py` or a CI job and
  nothing else. ⭐ when you add a check, name the TIER that runs it. ⚠ the
  omission had hidden two suites red on `main`, repaired in `ea5ca88df`.

- ✔ **D159 — CLOSED 2026-08-17. A name plate printed through a taunt because
  a speech bubble was a FOURTH label family that never joined the one
  placement pass.** `WorldLabelFamily` becomes `Signage · Fixture · Actor ·
  Speech`, ranked LAST by the module's own test (which family can move
  without anything visibly jumping?). D158's mechanism, its constants and
  `PendingSpeechBubble` go with it.
  ⭐ two defects fell out of the same shape: a 160-unit point radius gated
  stacking for text rendering ~336 wide, and displacement advanced in a
  fixed 11px quantum so a budget of "six steps" bought two lines of
  clearance — both replaced by one `max_displacement_px`. Guarded by
  `a_name_plate_and_a_speech_bubble_do_not_print_through_each_other` and
  `the_bubble_yields_to_the_name_plate_and_not_the_other_way_round`.

- ✔ **D155 — CLOSED 2026-08-16. Nobody got launched — two bugs on the shared
  floor, not the parameter tweak it looked like.** (1) every authored launch
  direction in the game was VERTICALLY INVERTED: `HitVolume::launch_dir`
  states `+y = gravity-down`, ~100 authored literals wrote against it, and
  `knockback_velocity` negated `y` to satisfy the opposite doc comment —
  every up-tilt/up-air/up-smash spiked victims into the floor. (2) a launch
  big enough to TUMBLE was resolved as a LANDING on the tick it was applied:
  the launched body kept its stale resting contact into the same step's
  `tick_knockdown`, read `on_ground == true`, and zeroed velocity — a 3269
  px/s launch moved zero pixels. Fixed by clearing `ground.on_ground` when
  `launch_into_tumble` returns true, gated on the tumble answer so
  `tumble_speed: 0.0` bodies (all of Ambition) are byte-identical.
  ⚠ every floor-game test set `on_ground = false` before launching, so the
  actual in-match situation — standing on the stage — was never stepped.
  Guards in `hit_response::launch_direction_tests`,
  `movement::tests::combat_actions`, `smash_in_the_host::launched`, each seen
  red pre-fix. Both fixes are in `ambition_platformer2d_core`, so Ambition
  gets juggling from the same floor.

- ✔ **D147–D154 — ALL EIGHT REVIEW FINDINGS CLOSED 2026-08-17.** (external
  structural review, 2026-08-16, read against `2381e3a7e`.) 6 of 6 reproduced
  when probed; D151 leaves one named residual, recorded on its own row below.

⚠⚠ **provenance, and it is the durable half of this row.** The reviewer states
they "couldn't independently run Cargo in this review environment" and
treated the commits' reported green suites as evidence rather than rerunning
them. ⇒ every finding was a READING, not a measurement, and each was probed
before being fixed — a finding that cannot be made to fail is a finding about
the reader, not the code.

- ✔ **D147 — CLOSED 2026-08-17 (`797aa480d`). Generic match activation knew
  the stocks ruleset's private latch** — D140's fix had inserted
  `StocksMatchSettled(false)` inside the GENERIC activation road, installing
  the resource even where that ruleset was never composed.
  ⭐ PROBED FIRST: the coupling was LOAD-BEARING (comment it out and match
  two ends with zero winners), so this REPLACED it. The latch now carries
  the `MatchInstance` it is about; a new match reads as undecided by
  construction. Guarded by `the_previous_matchs_verdict_does_not_settle_this_one`,
  `a_verdict_from_another_session_does_not_settle_this_match`,
  `adopting_a_seat_topology_does_not_un_decide_the_match`.

- ✔ **D148 — CLOSED 2026-08-17 (`f0da10217`). A team victory announced the
  last surviving teammate instead of the team** — the card decided which
  side's name to swap by COUNTING BODIES standing, but an eliminated fighter
  is despawned, so a team that lost a member early has one body at victory.
  Fixed using `PreparedMatch::seats_on_side` (how many a side HAS) rather
  than how many are standing. Guarded by
  `a_team_victory_names_the_team_and_not_its_last_survivor`.
  ⚠ the guard was rewritten once: the first version let CPUs fight freely and
  a hitlag change in another crate flipped the winner — a claim about a
  card's WORDING must not depend on combat tuning, so every elimination is
  now caused by the test on a fixed schedule.

- ✔ **D149 — CLOSED 2026-08-17. Move VFX bypassed `FxRequest`, so fourteen
  movesets hand-paired every sound** — `dispatch_move_events` wrote
  `VfxMessage::Effect` directly around the one abstraction that pairs an
  effect's visual with its sound. Of 145 authored `sfx(…)` calls, 74 merely
  restated the default pairing (deleted, verified against
  `effect_cue(FxId::new(effect))`, zero differed), 21 were `.loop` overrides
  (kept — ten would have gone silent otherwise), 50 were independent voices
  (untouched). Guarded by `a_paired_burst_is_heard_exactly_once`, which runs
  the real `dispatch_move_events` + `process_fx_requests`.
  ⛔⛔ the half-landing in between was a live feel bug: one commit switched
  the arm and left the 74 restatements standing, so every burst played its
  sound TWICE for one session — 412 app tests stayed green throughout.
  ⚠ measured residue, deliberately left: two moves throw the same effect at
  `±x` on one frame and are heard twice (a burst-count question, not a
  restatement).

- ✔ **D150 — CLOSED 2026-08-18 (re-opened and re-closed same day). A
  projectile changed allegiance when its firer despawned** — allegiance was
  reconstructed every tick from the firing `Entity`, so a fighter fires,
  loses their last stock, the body despawns, and next tick the shot turns on
  its own team.
  ⭐ the shot's PRESENTATION half already had the answer
  (`inherit_projectile_presentation_sources`: "the bolt routinely outlives
  the body that fired it, so the source is STAMPED at spawn"). Same
  treatment for allegiance: `ProjectileAllegiance { faction, team }`, frozen
  on the first tick of flight, registered rollback state. ⚠ the grudge is
  NOT frozen (a feud is something the firer holds now); the faction stamped
  is the authored one.

  **Audit of every attack-authorization still recomputed from the resident
  firer**, prompted by the review: faction+team now stamped; the self-hit
  guard is fine; KO credit (`attacker: owner_entity`) is open, D148's
  neighbor; the grudge read stays live and is correct
  (`dissolve_settled_grudges` already ends a feud on a health rule, not
  residency).
  ⛔⛔ found live in the audit: `indiscriminate` was `allegiance.is_none()`
  while the comment beside it said "a bolt that never had a living owner" —
  different sentences. A named firer that vanished before the stamp landed
  was promoted to environmental hazard PERMANENTLY (re-asking and re-failing
  every tick). Fixed: `indiscriminate` now requires no owner was ever NAMED.

  ✔✔ `stamp_new_projectile_allegiance` takes the side where the entity is
  BORN, installed as a monolith-side combat-chain system (the presentation
  stamp lives in `ambition_projectiles`, which depends on neither
  `ambition_combat` nor `ambition_characters`).
  ⛔⛔ needed TWO placements, not one — `Materialize` runs before `Settle` in
  the same tick, so a fighter eliminated on the tick they fire loses the
  body after the bolt exists and before any pre-step placement sees it.
  Second placement added right after the player materializer.
  ⭐ guarded by `a_shot_stamped_at_birth_survives_its_firers_elimination`
  (uses `run_system_once` to model the exact tick — a plain `app.update()`
  can't isolate it).
  ⛔ a NEW rollback name reddens TWO baselines:
  `scripts/baselines/rollback-schema-baseline.json` and
  `game/ambition_app/tests/rollback_schema_baseline.txt` — the second a
  per-crate run never sees.

- ✔ **D151 — CLOSED 2026-08-17 (`d21031fc4`). `MatchAbilities`' `None →
  permitted` bridge turned PERMISSION into a GRANT** — with `permitted = kit
  + wall jump` so one character keeps an authored wall jump, every
  UNAUTHORED character took the `None` arm and got it too.
  ⛔⛔ PROBED FIRST: the bridge was LOAD-BEARING — the versus duel's
  `at_most`, neither duellist authored abilities, so deleting it naively
  would leave both with nothing. Safe order: dress the cast first
  (`VERSUS_FIGHTER_KIT`, `at_most(…)` now referencing that constant), then
  `apply` reads `authored.unwrap_or(AbilitySet::NONE)`. `levelled` is
  untouched, so smash's fourteen fighters are byte-identical.
  ▢ **still open**: a ceiling must NARROW the body's own kit rather than
  REPLACE it, which `apply` cannot do because it never receives that kit —
  that signature is the real remaining work. Then guard that no seated
  character relies on the bridge, so the next unauthored fighter fails
  loudly.

- ✔ **D152 — CLOSED 2026-08-17. Empowerment expiry was a per-game scheduling
  footgun** — a game had to install `run_empowerments` itself or a
  two-second invulnerability became PERMANENT; five adopters had each
  remembered, and smash's respawn protection is how it surfaced. Split:
  contact-harm interpretation stays a ruleset choice, ticking/expiry is a
  domain invariant now installed by the engine in a named
  `EmpowermentExpiry` set, at `GameplayEffects` (last of three phases,
  ordering-preserving). Guarded by
  `a_timed_empowerment_ends_in_a_composition_that_scheduled_nothing`.

- ✔ **D153 — CLOSED 2026-08-17. A missing required sprite page failed OPEN**
  — an absent page logged `error!` and `continue`d, letting the barrier
  report Ready with missing presentation revealed. `RoomAssetManifest`
  gained `unresolved: Vec<String>`; readiness counts each entry as
  settled-and-failed, refusing the reveal. Guarded by
  `a_required_page_the_realization_lacks_refuses_the_room`.

- ✔ **D154 — CLOSED 2026-08-17 (`97a5b76ea`). Authored VFX was only half
  body-local: position was transformed through facing/gravity frame, but
  artwork drew world-upright** — a left-facing fighter's `air_slice` landed
  in the right place pointing right. `FxPose` now rides the
  event/request/message, derived from the same two authorities (owner's
  frame, move's committed facing) as the offset, matching the angle the
  sprite renderer already stands a body up with. Identity is the default and
  all eleven emitters state it explicitly.

- ✔ **D140 — CLOSED 2026-08-16. A second match never started and never
  ended: "GO!" stayed up and nothing could win.** (Jon, reproducible — his
  own "I thought we had tests for that" was the finding.) Two defects met on
  one `if`: (1) `StocksMatchSettled` could not be RETRACTED between matches,
  so match two opened wearing match one's verdict — now retracted by
  ACTIVATION. (2) the announce card had two writers and no arbitration; the
  GO! card overwrote the victory card — the old guard protected the wrong
  half ("do not CLEAR the winner's card"). The ceremony now stops talking
  the moment the match is decided.
  ⭐ the sim clock is requested to `0.0` while a match is settled and `1.0`
  while one is live, safe because hitstop's sink reduces by `min`.
  ⛔ the guard is the SEQUENCE — two matches in ONE app. A test that builds a
  fresh app per match cannot fail this, which is why the existing ones
  passed.

- ✔ **D143 — CLOSED 2026-08-18. The stage's unarmed declaration reaches the
  seat; the publisher was reading its own deferred write.** (found while
  answering Jon's moveset census) `DeclaredCombatRules::unarmed_melee` was
  installed by the same system fifty lines below the read, through a
  deferred `Commands::insert_resource` — so on the frame the match is
  decided the resource did not exist. Fixed: `smash_declared_combat_rules()`
  is the one source now, publisher takes the floor from the value it's
  about to declare and inserts that same value (reading the resource would
  also have been wrong once it existed — on a second visit it holds the
  PREVIOUS match's declaration).
  ⭐ measured three ways in the shipped host
  (`smash_in_the_host::report_what_an_unarmed_fighter_swings_once_the_stage_has_armed_it`):
  `mary_o`, `sanic`, `npc_alice`, `npc_bob` had zero moves before the fix.
  ⛔⛔ the guard that should have caught this SUPPLIED the missing value
  itself — `the_match_gives_every_seat_a_kit_that_can_hit` used to pass the
  swipe in by hand; it now calls `smash_declared_combat_rules().unarmed_melee`,
  poison-verified. ⛔⛔ a test seating a peaceful fighter cannot be written
  today — D144 armed every selectable fighter, so the floor has no live
  subject; the guard is necessarily about the mechanism, not a character.
  ⚠ what is left is not plumbing: whether the peaceful cast should be armed
  by the stage at all, or re-authored as fighters, is filed in
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §26 —
  the defect was real and fixed under either answer, but the next kit-less
  character seated finds it again.

- ✔ **D144 — CLOSED 2026-08-16. Every selectable fighter now has the full
  sixteen-press smash kit** (robot v3 12→16, goblin and the Oni 11→16, the
  automaton 8→16, Mary-O/Sanic/Alice/Bob 0→16). The up-B was the half that
  mattered: several fighters had NO special at all, no way back to the
  stage.
  ⛔ the census was wrong twice: asking whether a verb KEY exists reads a
  fallback as coverage, and asking only one posture invented a gap (George
  Booul's down-B is `airborne_only` by design). Fixed: a press is covered
  when SOME posture reaches a move of its own.
  ⭐⭐ a special owes an answer in BOTH postures (Jon: *"a down-b that has
  special airborne properties should also have an effect on ground — think
  Bowser"*) — `directional_verb_chain` already puts `special_air_down` ahead
  of `special_down`, so a two-form move is authored, not engineered.
  ⛔⛔ this changes nothing in Mary-O's or Sanic's own games — a move table
  is what the swing IS, the ability is whether the body may swing at all.
  ⭐ the census is a ratchet, `report_the_smash_kit_every_selectable_fighter_has`,
  reading its target from `SMASH_KIT.len()`.

- ✔ **D145 — CLOSED 2026-08-16. No projectile could hit anybody on the smash
  stage — melee and projectiles asked different questions about who may be
  hit.** Melee used `team_allows_damage`; the projectile loop used
  `damage_lands(firer_faction, victim_faction)` with no team, ever — both
  seats came back `ActorFaction::Player`, so every shot from every fighter
  was spared as an ally. Fixed with one call to the existing
  `damage_lands_between`, since `StrikeVictim` already carried the victim's
  `team`. Guarded by a fixture with the poison inside it: a body on the
  firer's own team, overlapping the same shot, that must not be hit.

- ✔ **D142 — CLOSED 2026-08-16. A match could only ever TAKE verbs away, so
  no stage could promise a fighter anything.** Jon: *"in smash all
  characters should be sure they are granted the basic smash abilities, but
  we want to do this in an elegant way."* Fixed: `MatchAbilities { granted,
  permitted }`, `effective = (authored ∪ granted) ∩ permitted`. Smash
  declares `levelled(SMASH_FIGHTER_KIT)` (granted == permitted); versus
  declares `at_most(..)`, the lone mask it always was.
  ⚠ one fighter changed in play: the automaton gains double jump, fast
  fall, dodge, pogo and ledge grab under smash's grant. Sanic authors
  `[RunJump]` on both iterations — his super form is speed, not a
  capability unlock.

- ✔ **D141 — CLOSED 2026-08-16. One fighter on the smash grid could not
  grab a ledge, and the ledge lived at home on two who should not have it.**
  The Perfect Cellular Automaton's authored kit was written for the duel
  arena on `AbilitySet::basic()` (ledge = false), and `fighter_abilities`
  was an INTERSECTION so the stage could not give it back. Two roads: a
  character's own game reads the catalog grant list; a smash seat reads the
  character definition ∩ the match's mask.
  ⛔ a row that authors NOTHING falls through to `sandbox_all` — Sanic was
  carrying ledge grab, swim, glide, dodge and a bubble shield invisibly
  around his own speedway. His row now authors `[SaneSubset]`, excluding
  those five by name.
  ⚠ what a runner's kit should actually be is still open.

- ✔ **D138 — CLOSED 2026-08-17, CONFIRMED BY JON: "Oiler fights in his new
  body now."** Body swap (`69eee645f`) moved him off the Python-drawn sheet
  onto the direct-SVG rig; kit followed (`bd6cbf775` + `95b45b6cc`, sprite
  submodule `3f7d265`) — sixteen moves, eight new side-view rig clips,
  eighteen of twenty-three effects bound, geyser as Up-B. Carried with one
  new authoring primitive (`moveset_authoring::strike_tag`) and no engine
  change or character-ID branch.
  ⛔⛔ **A SHEET SWAP OWES THREE REGENERATIONS, NOT ONE** — after the rig's
  sheet installed, `ultrapack.json` still carried Oiler at the TOON frame
  size, since tier atlases bake from whatever was in `$sprites_dir` when
  last packed. Run `--target <t>`, then
  `regen_visual_quality_variants.sh --target <t>`, then the four ultrapack
  tiers.
  ⛔ author arm poses as ANGLES on the reachable circle, never as (x, y)
  guesses — Oiler's arms are 26.5px from a shoulder 25.6px above the hanging
  wrist, already near full extension at rest.

  ⭐ four cues derive a plain id (`oil_geyser_stream`, `invariant_loop`,
  `gate_calibration`, `portal_leak`) while the bank ships only the
  `.loop`-suffixed version; measured 2026-08-18: one live (already
  overridden via `vfx_cued`), three latent (zero Rust references today).
  ⛔⛔ the two obvious fixes both cost something: a guard that every derived
  cue must be shipped would need an exemption list for Oiler's own two
  re-strikes (an exemption list is a TODO list); falling back to
  `{cue}.loop` would retrigger the loop on those same re-strikes.
  ✔ **DECIDED 2026-08-19 — JON PICKED: LEAVE IT RECORDED, no guard, no
  fallback.** ⛔ this is decided, not deferred: do not propose the guard
  again unprompted.
  ⚠ a BALANCE pass with real eyes: Oiler lost the observed match 36% to
  5%, the design's direction but a bigger margin than intended.
  ⚠ **why this row was stale for a day**: `regen_sprites.sh` still listed
  `oiler` in `review_cues`, which would have overwritten the rig's sheet on
  every full run. That entry is now an explicit ⛔ refusal pointing at
  `tackon_targets`.

- ▢ **D125 — The systemic world substrate: what a thing IS, which occurrence it
  is, why it exists, and how long it lasts.**

⭐⭐ **A LIMBED MOUNT CHAIN CLOSES THE LEDGER FOR FREE.** `gnu_ton_arena` authors a
boss riding a mount that has hands: possessing the boss makes the chain
**rider → mount → limbs** — three links, two relation kinds (`RidingOn`, then
`Limb`). The chain is marked `InCustodyOf`; `project_custody_onto_authored_occurrences`
records every marked room-scoped occurrence as `InCustody`; a room rebuild
consults that outlook and declines to author what somebody is holding. Measured
in `gnu_ton_arena` while riding: all four identities appear — the rider, the
mount, and both limbs. So nothing in the ledger is per-relation, and a fifth
attachment kind inherits the protection by being an edge.

⇒ **the closure iterates to a FIXPOINT instead of walking a fixed depth**: edges
are `(attachment → anchor)`; an attachment travels when its anchor does; iterate
until nothing changes, bounded by the edge count. Poison-verified: dropping the
limb edge fails only the limb test.

⚠ **`CapturedBy` is deliberately NOT an edge.** A captive is attached to its
captor by exactly this rule, but no composition can express a captor carrying one
through a door — capture is the platform fighter's, and a versus stage has no
room changes. ⛔ adding it would be a rule for a state nothing can reach. ⇒ **if a
room-based game ever gets grabs, that is the edge to add, and this clause is the
note that says so.**

⭐⭐ **A POSSESSED ACTOR WAS DUPLICATING ITSELF — FIXED 2026-08-19.** Possessing an
authored enemy and carrying it through a door left two live entities behind one
`SimId`: `record_placed_ground_items` is the only writer of `AuthoredOccurrences`,
so the occurrence ledger was items-only and never recorded where a possessed body
went. Possession expressed "this body travels" by SWAPPING ITS LIFETIME (remove
`RoomScopedEntity`, insert `SessionScopedEntity`) while items suspend residency
instead (`InCustodyOf`, lifetime untouched) — two mechanisms for one idea, and the
ledger's projection was blind to the swapped one.

⭐ Fix: possession now keeps the room scope and adds `InCustodyOf`; `RoomResident`
is `(With<RoomScopedEntity>, Without<InCustodyOf>)`, so the custody marker alone
excludes a travelling body from a room change's sweep. The item domain's own
custody projection (`project_custody_onto_residency`) had been using
`Has<RoomScopedEntity>` as a proxy for "would a room change retire it" — true only
while there were two kinds of holder; a possessed body is a third (room-scoped AND
travelling), so it now asks the `RoomResident` roster instead, making custody
TRANSITIVE for free. ⚠ two queries, not one: a despawned holder's custody is
deliberately left dangling (a known remaining orphan), so conflating "gone" with
"travelling" would make that orphan follow the player through every door forever.

⭐ Three possession unit tests pinned the old mechanism and are rewritten to
assert *scope untouched, custody added and dropped* — the poison for
reintroducing a promotion, since a promote/restore pair looks identical at the
end and differs in the middle. The old promotion also let a possessed enemy from
a destroyed world survive into a new game still being driven; the fix removes
that too.

⭐⭐ **THE SAME BUG HAD A THIRD POPULATION: a mount ridden through a door was
destroyed** (measured in `pirate_sky_lookout`/`pirate_cove`) — a mount is an
ordinary authored room actor and nothing suspended its residency. The rule is
TRANSITIVE: a mount is in its rider's custody exactly while that rider is itself
travelling, and `RoomResident` answers the question at every link without
anything counting them. ⚠ the discriminating case is the negative one — an
AI-piloted sky rider is room furniture and keeps its mount, so a rule that gave
every mount to its rider unconditionally would stop every authored mount from
being retired with its room.

⛔⛔ two wrong-first attempts, kept for the lesson: (1) two owners wrote the same
fact on the same tick — a mount projection in `WorldPrep` granting custody and
possession's in `PlayerSimulation` retracting it, with no structural filter
separating the populations — fixed by deciding the whole non-item body population
in one pass, one component, one owner; (2) the rule first asked `RoomResident`
about the rider, a roster that EXCLUDES anything already wearing the marker the
system itself writes, so the chain converged one tick per link and a released
rider left its mount in custody for a frame — fixed by asking `RoomScopedEntity`
instead. ⚠ `bevy::log::warn!` inside the affected system printed nothing during
diagnosis because the default log filter swallowed the custom target — worth
remembering before trusting a silent log as "not reached".

⛔⛔ **the first fix was itself wrong in a way only rollback would show.**
`InCustodyOf` is a DERIVED component excused from the snapshot by "room residency
reprojected from `ItemCustody` every tick" — but a possessed body has no
`ItemCustody`, so writing the marker at the possess site created a population
nothing reprojects: a rewind past the possession drops it and the body silently
becomes a `RoomResident` again. Fixed by making the marker a projection of
`PossessionState` (which IS snapshot state), so the derived-component excuse is
true for both populations. Poison-verified: moving the write back to the possess
site fails only the rewind test.

⚠ one measured consequence, left as-is: a driven body enters `CustodyBaseline` as
`placement:… <- slot:0`, which the item domain's restore ignores (its loop is
keyed on live items) — harmless, guarded by
`a_checkpoint_taken_while_possessing_does_not_manufacture_an_item`.

⭐ the save/load consequence was checked rather than assumed: a driven body's
occurrence enters the ledger as `InCustody`, and a save taken mid-possession
would carry that row to a fresh process where nobody possesses anything. It does
not survive: `republish_custody`'s retract-by-resetting contract runs every tick,
ungated, and the row is gone one tick after possession clears — guarded by
`a_custody_row_with_nobody_holding_it_is_retracted_before_a_room_can_act_on_it`
(asserts both that the row is written at all, and that it is retracted).

⭐ possession was the only subsystem doing this — grepping the capability
(`remove::<RoomScopedEntity>` + a later `SessionScopedEntity` insert on an
already-created entity) across every crate finds one other hit,
`SessionSpawnScope::apply_to`, which is spawn-time ownership rather than a
promotion, so there is no second place with the same ledger blindness.

⚠ `PossessionState::restore_scope` is vestigial now and deliberately kept — it is
a field of a rollback-registered resource, so retiring it is a schema change that
belongs to a version bump, not to this fix. Release now touches no scope at all;
removing and re-inserting one would be a bug, since it could silently revert a
scope write some other system made while the body was driven.

⇒ guarded at three tiers:
`an_authored_actor_carried_out_of_its_room_and_back_does_not_meet_a_copy` (app,
the duplication), `an_item_carried_by_a_possessed_body_survives_the_door_too`
(app, the coupling), `an_item_held_by_a_possessed_body_travels_with_it` (unit,
the transitive rule).

⚠ **still open — two named pieces.** An actor RELEASED in a foreign room writes
no `Placed` row, so it is retired when that room is left and re-authored at home
on re-entry — defensible ("the enemy goes home"), but keeping it where it was
left needs BOTH a producer (the placement recorder is items-only; a released body
needs the same `republish_placements` call) and a consumer
(`construction::relocate_request` returns FALSE for anything but a ground item
today, so an actor request would be refused and rebuilt at its authored spot with
a warn). ⭐ the refusal is honest, not broken — the room build already declines to
pretend an unmovable family moved. ⛔ adding the producer without the consumer
would make every re-entry log a warn and teleport the actor home anyway; the two
land together or not at all. Whether an abandoned enemy should stay put is a
product call, not a defect.

✔ **THE PERSISTENCE CARRY LIST IS CLOSED — 2026-08-19.** A checkpoint
baseline used to require five hand-synchronised enrollments across three crates;
`OwnedItemsBaseline` missed durable adoption and demonstrated the failure. The
runtime now composes typed lifecycle + actor checkpoint plugins, each domain owns
its concrete baseline systems and rollback facet, and `DurableBaselines` is
deleted. Durable item adoption happens in the same restore that applies the saved
bag, and a final completion step raises the global restore latch only after every
domain adopter runs. That also fixed the hidden pre-load-bag ordering bug with a
fresh-process poison. No erased callback registry was introduced; see
`engine/instance-lifetime-provenance-and-persistence.md`.

⭐⭐ **THE CONSTRUCTION FEDERATION'S SECOND LANE LANDED CHEAPLY — 2026-08-19.**
Gravity zones (chosen because they are boring — not an actor, no relation
vocabulary, no execution services) moved out of `ActorConstructionParams` into
`shared_tangle::gravity::construction`, beside `GravityZone`/`OscillatingZone`.
Zero new dependency edges (the construction machinery and `SpawnSessionScopedExt`
are already in the same crate); the constructor takes RESOLVED parameters, not
`GravityZoneSpec` (which lives downstream in `ambition_platformer2d_world`);
`GravityPlugin` contributes catalog metadata exactly as `PortalGunPlugin` does.
The lane is not feature-gated, unlike portal-gun, proving the same composition
shape applies to a capability every composition has.

⚠ the cost, recorded because it is the number that will justify a different
composition shape someday: a second lane touched ELEVEN separate blocks of
`RoomFeatureConstructionPlan` (plan field, receipt field, prepare, roster claim,
struct literal, deterministic dump, binding assert, verify, rebuild-one, commit,
committed ids). ⛔ it does not yet justify type erasure — a universal registry
able to execute a new domain is what this seam exists to avoid. The cross-lane
collision check, previously a hand-written pairwise intersection, is now a fold
that claims each lane's ids into the roster, so composing a lane and checking it
are the same line.

⭐ **portal-gun lane installation vs. schema fingerprinting is settled**: two
authorities agree by composition, not by type. The executable lane is compiled in
by `#[cfg(feature = "portal")]`; the schema fingerprinting entry is contributed by
`PortalGunPlugin` at runtime. A composition compiling `portal` but installing only
`PortalSimulationPlugin` would fingerprint a gun-less world while its rooms still
built authored gun pickups — prevented by one line: `PortalSchedulePlugin`
installs `PortalPlugin` (simulation plus gun) and is the only place in the
workspace that installs portal simulation at all. A test in `portal_schedule.rs`
pins that coincidence and goes red the day the line changes.

⭐ **A THIRD INSTANCE ARRIVED 2026-08-17, from a domain this row had not
touched, and it is the cleanest statement of the row's thesis: D150.** A
projectile's allegiance was reconstructed every tick by querying the firing
`Entity`, so a shot in flight turned on its own team the moment its firer
despawned — body RESIDENCY was standing in for stable identity. The same
domain's presentation half had already solved this
(`inherit_projectile_presentation_sources`: *"the bolt is the emitter … routinely
outlives the body that fired it. So the source is STAMPED at spawn rather than
looked up at impact."*); only the combat half still counted who was standing.
⚠ D148 was the same error the same day — the winner banner decided "is this side
a team" by counting resident bodies, so a team whose other member had been
eliminated announced its last survivor's name. ⇒ three independent sites in one
campaign, none aware of each other, each fixed by asking a FROZEN record instead
of a live query — the argument for the substrate rather than three more point
fixes.

✔✔ **THE RESTORE FALSIFIER IS GREEN (2026-08-16, `13dd4d31b`)** — bank a reward
at a checkpoint, carry it to another room, drop it there, leave so that room
unloads, then die: it comes back into the hand that banked it, as the same
occurrence (`SimId` and `SpawnOrigin` from the authored record), with its
pedestal still empty and no duplicate. Driven end to end on the composed host
through authored LDtk items, a real `HealShrine` + `Interact`, real door
crossings and a real `ActorDiedMessage`.

⭐⭐ the missing mechanism was MATERIALIZATION: the custody restore was pure
*re-assignment* — it walked live objects and asked whether the checkpoint agreed
with where each one was, a question that cannot be asked about an object whose
entity no longer exists, and no room build could supply it either (an
`InCustody` row makes `outlook_for` answer `Suppressed` in every room, because a
thing in a hand is not a thing in a room). ⇒ every other reconstruction road in
this engine starts from a ROOM and asks what it owes; this one starts from an
occurrence resident in no room, so the authored definition has to be reachable BY
IDENTITY. No new rollback state; schema stays v32.

✔✔ **AND THE RUNTIME-MINTED CASE CLOSED TOO (2026-08-16, `88b611caf`).**
Materialization was bounded by "some room authors a record with this id"; a
runtime-minted instance (the throw's `SimId::spawned` arm, where the inventory
count table equips an item with no object behind the hand) had no record
anywhere and was lost on a death. The minimal durable description turned out to
be three things and no more:

```text
identity     the occurrence's own SimId
provenance   SpawnOrigin::Dynamic { parent, sequence }
definition   the item spec's authored id — a REFERENCE, never a copy
```

⛔ no position, no velocity, no component snapshot — that is rollback wearing
save's clothes. `ground_item_physics` refuses to step anything not `InWorld`, so
the hand supplies the place.

⭐⭐ the first prediction was one field short, and the missing one is the
durable-save lesson: `(identity, spec)` alone rebuilds an instance that cannot
say which spawner it descends from (the state `SpawnOrigin`'s doc refuses to let
anyone spell), so it would survive exactly one death and then be invisible to the
next capture. ⇒ a durable description that restores the thing is not sufficient;
it must restore the thing's ABILITY TO BE DESCRIBED AGAIN — identity and
provenance are now minted as one value.

⭐ snapshot-not-registry is MEASURED, not asserted: rebuilt as a growing registry
of every mint with the restore returning each row to its spawner's hand, the
banked-item fixture stayed GREEN and
`a_runtime_mint_the_checkpoint_never_saw_is_not_resurrected_by_a_death` went RED.
`MintedItemBaseline` answers HOW to rebuild; the custody baseline still decides
WHETHER and INTO WHOSE HAND — it lives in the item domain because the lifecycle
crate cannot see a `GroundItem`'s spec. Schema 32 → 33.

⛔⛔ **AN INSTRUMENT DEFECT: A FIXTURE HAD BEEN MEASURING A WORLD NOBODY CHOSE.**
`with_start_room` took a ROOM ID, but `central_hub_basement` (used throughout one
test's comments) is an LDtk LEVEL name; the option silently fell back to the
authored entry room instead of failing. Measured blast radius: of 40 literal call
sites, only that one didn't resolve as a real room id, plus one deliberate
negative test.

⚠ a written counter-argument in the code
(`game/ambition_app/src/app/resources.rs:~240`) argued the strict/tolerant split
was deliberate: a programmatic override from a library caller
(`Platformer2dSimHarness`) may legitimately miss, so falling back is correct,
while a CLI flag is already strict because it was typed by somebody who wanted
that room. The objection was answered by counting adopters: tolerance had zero
beneficiaries — every literal in the tree was a real room id.

✅ **LANDED 2026-08-17 (`3f116e88b` seam, `90187a559` migration).**
`with_start_room` stayed tolerant (a test, `unknown_start_room_does_not_panic_or_error`,
makes tolerance a promise) and `with_required_start_room` was added beside it,
refusing to boot and listing all 72 ids. All 24 files that name a room —
including the shared `tests/common::fixed_60hz_room_options` helper (fanning out
to ~20 more fixtures) and the `ambition_app_tools` `headless`/`rl_smoke` binaries
— were migrated to the strict form; one tolerant literal remains, which is the
promise's own negative test. `cargo test --workspace --lib` 4867/0, `app_it`
412/0. The defect this item was opened for is gone: the test that asked for
`central_hub_basement` now names a real room.

⛔ one literal wasn't a room id at all: `collision_invariant_oracle::run_episode`
used `""` as its OWN sentinel for "keep the LDtk-authored start" — never a
beneficiary of the fallback. Fixed by passing no start room rather than asking
tolerantly for one. Nothing else in the tree (every `const ROOM`/hand-listed room
array) relied on the fallback — the row's premise held. A stale comment in
`rl_smoke` describing a now-unreachable fallback branch was corrected.

⭐ **PROMOTED FROM THE RESERVOIR 2026-08-14** — seven focused plans for this
frontier already existed and were reachable only from
[`tracks.md`](tracks.md), not from this queue:
[`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md),
[`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md),
[`engine/capability-progression-and-world-gating.md`](engine/capability-progression-and-world-gating.md),
[`engine/platformer-navigation-and-reachability.md`](engine/platformer-navigation-and-reachability.md),
[`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md),
[`engine/persistent-actors-and-population.md`](engine/persistent-actors-and-population.md),
[`engine/agentic-character-runtime.md`](engine/agentic-character-runtime.md).

**Sequence — each step's identities are what make the next one expressible:**

1. ✔ **The substrate is already built, under names this plan does not use —
   measured 2026-08-14.** ⛔ do not build it.
   - *What authored thing is this?* → `WornCharacter(CharacterId)` on the body,
     resolved through `PreparedCharacterRegistry`; its doc splits NAMING a
     template from APPLYING it (`RecharacterizeBody`).
   - *Which runtime occurrence is this?* → `SimId`: deterministic, namespaced
     (`placement:` / `slot:` / `encounter:` / `spawned` / `strike`),
     `#[require(SimIdCounter)]`, dynamic spawns minted as `(spawner SimId,
     per-spawner counter)`. Every snapshot row, checksum projection and
     cross-reference keys on it.
   - *Why does it exist?* → `SpawnOrigin`, a component:
     `Authored{source,instance}` / `ProviderStaged{provider,room,instance}` /
     `Dynamic{parent: SimId, sequence}`, `parent` non-optional by design,
     verified against the construction plan roster and encoded into rollback
     blobs. Its module states the rule outright: provenance is data, never
     recovered by parsing an id string.
   - *How long should it last?* → four ENFORCED scopes, each owning a sweep:
     `RoomScopedEntity`, `ModeScopedEntity`, `RoundScopedEntity`,
     `SessionScopedEntity`, plus per-domain TTLs and `EncounterCleanupPolicy`.
     `round.rs` states the rule: "round scope is a LIFETIME, not a provenance —
     where an entity CAME FROM does not say how long it should live."

   ⛔⛔ **the one real gap was a FALSE DECLARATION, now deleted.**
   `RunScopedEntity`/`PersistentEntity` (with `spawn_run_scoped`/
   `spawn_persistent`) had zero producers, zero consumers, and no sweep read
   either marker — worse than a missing lifetime, because `lifecycle/mod.rs`
   directed new spawn sites to `SpawnScopedExt` and two of its four verbs
   silently did nothing. `RunScopedEntity` duplicated `SessionScopedEntity`;
   `PersistentEntity` was a second spelling of absence (every sweep culls on
   marker presence, so an unmarked entity already survives all four boundaries,
   and the marker falsely implied it OVERRIDES a scope). ⇒ the surviving rule:
   **a scope is spelled here only if a sweep enforces it.**

   ⇒ **what genuinely remains in step 1**, both listed as unresolved by the
   focused plan itself: **persistence policy** and **the explicit terminal
   transition**. ⛔⛔ "has no runtime cleanup scope" does NOT mean "durably
   persistent open-world object that is correctly saved and restored" — an
   unmarked entity merely survives this session's four boundaries; the
   durable-persistence system is still undesigned.

   ✔ the player-centrism smell in `room_transition/commit.rs` is gone
   (2026-08-19) — whether a transiting body survived the room change was
   inferred from a presentation-state proxy that only ever bought nothing or
   lost; now an unconditional `Some(subject)`, safe because `retire_outgoing`
   already skips an entity absent from its roster.

   ⛔ authored identity does NOT imply world uniqueness — "there is normally one
   Fia" is content policy, not the meaning of a definition id. ⛔ do not invent a
   universal `EntityId`; the separation between identity types is settled and
   deliberately not unified.

2. ◐ **Item custody.** ⛔ "item entities carry no `SimId`" was WRONG on the
   authored path — `construction::authored_ground_item_requests` builds every
   LDtk `GroundItemSpec` row with `SimId::placement(&spec.id)` +
   `SpawnOrigin::Authored`. The real defect was that the identity WAS destroyed
   at the first custody change: pickup called `despawn()`, throw called an
   unconditional `spawn_room_scoped`.

   ⛔⛔ the pickup fork was SYMMETRIC — neither population was correct.
   `collect_ecs_pickups` used `With<PlayerEntity>` (N couch seats, excluding a
   possessed body — the reported bug); `collect_world_items` used
   `Res<ControlledSubject>` (exactly one body — so seat two never picked up
   mushrooms, an unreported half). Unifying onto `ControlledSubject` alone would
   have cut couch collectors from N to 1. Fixed with a filter-plus-value
   population serving both, pinned by a test asserting a second seat AND a
   possessed body both collect (either half alone passes on the broken code).

   ⇒ **the instance / quantity / consumable split is now written on
   `ItemCustody`:** a `GroundItem` is an INSTANCE and keeps its identity across
   world → held → world; `PickupKind::Currency`/`Health` and the `OwnedItems`
   counts are QUANTITIES (two coins are the same coin — what survives is a
   number on the collector); a `WorldItem` is a CONSUMABLE whose despawn is a
   real end of life. A body equipped from the count table has no object behind
   its hand, so throwing turns a quantity into an instance, minted
   `SimId::spawned(thrower, counter.next())`.

   ⚠ `ItemCustody` IS rollback state, registered as such (clone + entity-SET
   probe since `InWorld` names no body, paired with `rollback_map_entities`);
   schema 29 → 30, both baselines updated. It gates drawing, physics and
   grabbability on later frames, taking over a job GGRS previously did through
   the entity anchor.

   ⛔ **the INVENTORY leg is explicitly NOT closed**, and says so in code:
   `OwnedItems` is a global count table with no row per object, so *whose
   inventory does a possessed body fill?* still has no answer, and
   `equip_held_spec`/`unequip_held` are labelled a migration seam rather than
   the model. ⭐ physical custody belongs to the body and the item instance;
   participant entitlement is a separate fact with a different owner and
   lifetime.

   ✔✔ **THE TWO WEAPON DROP SITES ARE MIGRATED (2026-08-19).**
   `damage/actor_hit.rs`'s dropped weapon and `damage/boss_hit.rs`'s signature
   gauntlet were the only two death drops the 2026-08-05 room-scope fix never
   reached — both spawned session-scoped with no `SpawnOrigin`, so a defeated
   pirate's gun-sword followed the player into the next room. Both now spawn
   through one `drop_held_weapon`. The coverage guard that should have caught
   this (`the_pickup_drop_table_is_complete`) scanned one file for one spelling
   and missed both inline `GroundItem` spawns; it now reads all four
   damage-path files for both collectible spellings, poison-verified to name
   `apply_actor_hit` the moment a drop is written inline again, and checks the
   production `RoomResident` roster rather than restating the rule.

   ⇒ **carried forward:** death drops still mint no `SimId`, deliberately — an
   identity would enrol the drop in `TransactionBaseline::capture`, whose
   roster a room-scoped entity leaves mid-transition; provenance without
   identity is the honest intermediate state. An orphaned custody is possible
   if a holder despawns while carrying (inert, bounded by room/session scope,
   no reaper built — no state has demonstrated the need). `ChestFeature::reward()`
   still has zero production callers — authored chest rewards are parsed,
   lowered onto the live component, and never granted.

3. ◐ **Capability-driven gating and platformer reachability — first slice
   landed 2026-08-14, driving the real kernel rather than a hand-enumerated
   capability list.** The design an agent arrived with (a closed-form reachable
   envelope with a hand-enumerated capability list) would have failed the same
   way as the deleted "airborne + below the lip ⇒ already dead" rule; instead
   `movement/recovery.rs` clones the body and drives its OWN kernel
   (`ae::step_motion`, pure, no Bevy `World`) over three fixed ordered efforts,
   reporting `Regained { steps, side }` or `NoSupportFound { reset }`.

   ⭐⭐ it states no rule about bodies — every capability the kernel implements
   is honoured because the kernel honours it, gated by the body's own
   `AbilitySet` and `AxisSweptParams`, so there is no capability list to fall
   out of date. ⚠ the effort is a reactive rule, not a search: hold the side,
   hold jump, re-press the instant the body stops rising — pressing every tick
   burns a whole air-jump budget in consecutive frames, pressing at apex chains
   the most height, holding between presses avoids cutting a variable-jump arc.

   ⇒ `recovery_capability_gap(..) -> Option<AbilityGrant>` answers "which
   capability blocks the route" in the engine's existing authoring vocabulary,
   skipping a grant that adds nothing (re-granting `AirJump` to a body that has
   the verb and spent the charge would refill the budget and misreport a spent
   charge as a missing verb).

   ⛔ **the consequence is open: nothing reads either function.** No call site
   outside the module's tests, nothing wired into the fighter's search, no
   fighter tuned. `NoSupportFound` carries `reset: Some(cause)` only when every
   effort ended in a world reset; the module doc says a brain, a validator, or
   an LLM decides what that means. Nothing added is rollback state.

   ⭐ the three pins, each with its falsifier: the body's own kit decides (same
   world/position/velocity, only `move_horizontal` differs, both terms
   asserted); the deleted rule's exact `doomed` state is answered by surfaces
   (poison: remove the one catch block, the identical body must report
   not-recovered); the probe is gravity-generic (room transposed, gravity along
   `+x`, non-steering body still fails).

   ⇒ **a deletion gate was named rather than taken**, on the fighter rollout's
   duplicate integrator (whose own doc concludes "the fix is not different
   constants, it is DERIVATION"). Delete it when three things hold: the brain
   can obtain a `&ae::World` without depending on `ambition_platformer2d_world`;
   one real kernel step per shadow step is measured affordable against
   `rollout_k × (1 + rollout_depth)`; and `ladder_rig --scenarios` re-runs green
   — the only instrument that has ever caught a shadow-physics divergence.

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
Instance-capability is decided solely by whether `Item::held_item_id()`
resolves — 9 held weapons are both an instance and a count, the other 5 classes
are counts forever whose readers legitimately want a quantity. ⛔ so do not give
the count table a row per object.

⚠ the flattening is `items/pickup/mod.rs:616` and composes into an unbounded
duplication loop (equip from the count with no object behind it → throw → the
mint arm materialises a second axe). ⛔⛔ not fixed, deliberately: `owned.take` on
throw destroys thrown items on room exit, because the count is currently the
durable-save mirror of an instance, not entitlement.

✔✔ **A CARRIED ITEM CROSSES A ROOM BOUNDARY WITHOUT THE TRANSITION KNOWING ITEMS
EXIST — LANDED 2026-08-15.** `ItemCustody` is projected onto an `InCustodyOf`
marker; the roster a room change retires became `RoomResident =
(With<RoomScopedEntity>, Without<InCustodyOf>)`. The item never loses its room
scope (so a *reset* still destroys it, correctly), and losing custody makes it
resident in whatever room is active then — room residency carries no room id, so
"dropped in the destination" needs no memory. It is body-generic for free: a
room-fixture holder (unpossessed NPC) leaves the item resident so it dies with
its room; a despawned holder makes it resident again, closing the death-drop
orphan; a possessed carrier gets the travelling answer without anyone asking who
the player is. Poison-verified: reverting the roster to plain
`With<RoomScopedEntity>` fails the carried-item test exactly and leaves the reset
test green.

✔ deleted in the same slice: `world/rooms/load.rs`'s `commit_room_transition_geometry`
+ `RoomLoadResult`, a public 60-line second copy of the room commit with zero
callers. `derived.custody_residency` is a declared-derived row (recomputed
unconditionally each tick, no "already applied" gate), so schema stays 31.
Verified: app_it 363, Smash 18, monolith 1230, contracts 27/27.

✔✔ **CONTINUITY'S FIRST LEG LANDED 2026-08-15** — a rebuilt room now asks what
became of the occurrence it minted last time, via a DISPOSITION rather than "is
something with this id alive": `OccurrenceDisposition::{Authored (default),
Persisting, Consumed}`. Construction gained a sixth stated authority
(`occurrences: Option<&AuthoredOccurrences>`), and
`RoomFeatureConstructionPlan::prepare` retains only requests whose disposition
`authors_a_fresh_occurrence()`. ⭐ `Consumed` is spelled and read but has NO
PRODUCER, deliberately — the honest slot for permanent destruction, making
"ephemeral / resettable" the default rather than a special case, reserved before
the terminal cases exist.

✔ deleted: `RoomConstructionPlan::prepare(&World, ..)` — 84 lines, zero callers;
`RoomConstructionError::MissingService` went with it. The prefetch cache now
states the dispositions it froze and refuses a plan prepared against different
ones; `ConstructionPlan::prepare`'s `live` argument (previously
`&Default::default()`, false since the custody slice) now carries the suppressed
set, so a road reaching a suppressed identity gets `IdentityAlreadyLive` at
preflight instead of a duplicate.

✔✔ **ITS OWN NEW TEST FOUND TWO FAULTS, BOTH CLOSED 2026-08-15 — one instrument,
one real.** Real: a sandbox reset rebuilt the room and then emptied it —
`process_new_game_reset_request` retires every `RoomScopedEntity` and commits the
fresh room plan, `.chain()`ed with `clear_transient_on_sandbox_reset`, whose
transient sweep then despawned the items the reset had just authored (authored
ground items are room-scoped). ⛔ a plain `.chain()` carries an auto-inserted
`ApplyDeferred`, so a later system in the chain SEES what an earlier one just
spawned. Fixed by deleting the overlap: the transient sweep gained
`Without<RoomScopedEntity>`, since `retire_outgoing` already sweeps room scope
unconditionally and is the stricter of the two — this silently fixed the same
latent defect for an authored `PortalGunPickup`. Poison-verified both ways.

⇒ **what a sandbox reset restores, stated once:** the start room from its
authored records alone — every authored placement, feature and actor comes back
exactly once, including one that was being carried, because the reset destroys
the world those occurrences live in, hands included. ⛔ it restores nothing
outside room scope: a dropped weapon, a summoned ally, a placed portal and the
player's held state are retired by the transient sweep and never come back.

Instrument fault: the test *"a carried object survives a reset"* never ran the
reset it described — `reset_episode()` triggers `reset_sandbox` +
`ResetRoomFeaturesEvent{Manual}`, which restores room FEATURE state in place and
never touches `RoomScopedEntity`. The real reset (`process_new_game_reset_request`)
is reachable only through `NewGameResetRequested`, whose writers are the
kaleidoscope menu and tests — nothing on the input path. ⛔⛔ the failure was half
silent: a sibling assertion (`occurrences(..).len() == 1`) passed vacuously,
since the surviving carried item WAS the one occurrence, so a reset that did
nothing at all satisfied it. The test now drives the real reset with every
original assertion kept, plus one the count couldn't see: the carrier's hand is
empty afterwards.

⛔ the deletion gate (collapsing the two reset roads) was refused on purpose —
`session/reset/mod.rs:165` already argues one sweep cannot answer both "does this
survive leaving the room" and "does this survive replaying it".

⚠ **a real latent gap found and deliberately NOT fixed:**
`clear_transient_on_sandbox_reset` scopes hand-emptying to `With<PlayerEntity>`,
so a non-player carrier keeps a dangling `HeldItem` after the reset destroys the
object — the simulation is not player-centric and this road still is. Left alone
because the same loop also restores `ActionSet`/`StashedActionSet` and strips
`PortalGun`, genuinely player concerns. ⇒ relaxing the filter is a product call,
not a refactor.

⇒ **remaining legs of the same question:** the two terminal cases — destroyed
permanently ⇒ never recreate (needs a `Consumed` producer) and intentionally
resettable ⇒ may recreate (already the default). ⭐ verified 2026-08-18: still
ZERO producers of `Consumed` — a feature waiting on a product answer (what
destroys something permanently?), not a defect. Two carried risks:
`SimId::placement(id)` is a global namespace whose uniqueness is checked only per
room, so two rooms authoring one id would suppress both; and the ledger is not
experience-scoped, so a suppressed row can survive into a new session.

✔ **THE FIRST RISK IS GUARDED (2026-08-18), while it was still free.**
`validate.placement_id_collision` warns when one authored `id` names things in
two rooms — green on every shipped world (twelve entity kinds carry an `id`; the
only cross-room reuse is `LoadingZone`, deliberate because a zone's
`target_zone` resolves within its `target_room`). ⛔⛔ the collision is
reachable, not hypothetical: `authored_logic/prepared.rs` turns an authored
`placement:<id>` argument into `SimId::placement(..)` in production, so the day
content names `placement:return_door` it would mean seven zones. ⚠ four tests
pin it, since a guard green on all real data is otherwise indistinguishable from
one that cannot fire.

✔ **the cross-world half is checked too (2026-08-19)**, via the RUNTIME's merged
`RoomSet` (the file validator alone cannot load every world at once):
`no_two_rooms_in_the_merged_world_author_the_same_id` — 72 rooms merged, 358
distinct authored ids, 0 collisions. Carries three falsifiers: a rooms floor
(≥50), an id floor (≥300, fired at 294 when one authored kind was dropped from
the collection), and the collision assertion itself. `LoadingZone` ids stay
exempt for the same reason as the file validator.

✔ **a suppression written in one experience survived into the next — fixed
2026-08-18** by `session::teardown::reset_session_scoped_resources_on_retire`
clearing `SaveRestored`, whose latch was set true in
`restore_inventory_from_save` and set false nowhere, so a second experience in
one process never re-ran the durable restore and inherited the first
experience's ledgers. Guarded by `retirement_clears_the_save_applied_latch`
(asserts both that the latch survives an ordinary frame and is cleared on
retirement).

⛔ two costed-and-rejected shapes: keying the reset on the session generation,
and a scope-watcher — both more machinery than the seam that already existed
(the lifecycle that ends the session is the one place that cannot forget). ⛔ the
exemption-list answer is also wrong: scopes are authored per experience, so
hand-adding `.resetting::<AuthoredOccurrences>()` to today's two experiences
would make a third game's omission a silent bug — the ENGINE owns the invariant,
the composition owns the order.

⇒ the hazard that started this leg, recorded on `ItemCustody`: carrying an
**authored** placement out of its room and back yields the carried object **and a
freshly authored copy with the same `SimId::placement(..)`**. It could not arise
while the boundary destroyed the object.

⭐ but the hazard is the small statement of a much bigger question, and this is
the systemic-world pressure the project has been trying to reach:

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
maintainer's rule hold end to end through production roads: an object acquired
before any checkpoint goes back on its pedestal; acquired-then-banked stays in
hand with the pedestal empty; and one death reaches two opposite answers about
two objects of the same kind in the same frame, separated only by which side of
the checkpoint each acquisition fell on — a result no `KeyItem => survives` rule
can produce.

⭐ the baseline is a projection of DOMAINS, not a resource: `lifecycle::horizon`
owns two messages and two sets and nothing else; `OccurrenceBaseline` and
`CustodyBaseline` are captured by their own domains from their own live
authorities. Both are checksummed rollback state (schema v32).

⭐⭐ three defects the fixture found by RUNNING it, not by reasoning:
1. restoring the ledger and emptying the hand DELETES the object — the room
   replay resets features in place and never re-runs authored construction.
   Fixed: a death is a checkpoint RESUME, recording the same
   `LifecycleIntent::Transition` a session-start resume records, so same-room
   re-entry rebuilds correctly.
2. custody is a FORKED relation (`ItemCustody` on the object, `HeldItem` on the
   body) — retracting one half left the body holding a ghost and refusing every
   future pickup. ⛔ the tempting generic repair ("empty a hand matching nothing
   in custody") would disarm every authored fighter, since a character
   definition's `held_item` needs no world object.
3. a hand must be EMPTIED before it can be FILLED — interleaved, the
   reinstatement was equipped over an occupied hand and `return_released_items`
   quietly undid it one phase later.

⭐ the gap that looked save-destroying is NOT, measured
(`a_banked_object_whose_room_unloaded_returns_to_the_hand_that_banked_it`,
`13dd4d31b`): a baseline row whose occurrence has no live entity (banked, carried
next door, put down, that room unloaded, then a death) would seem erased when
the restore overwrites the `Placed` row — but `republish_custody`'s
retract-by-RESETTING rule saves it in a case it wasn't written for: the custody
leg is rebuilt from live state every tick, so the unsupported `InCustody` row is
dropped and the home room authors the object at its pedestal. The player loses
the *acquired* property they banked (wrong but recoverable); the object is not
destroyed (which would not be).

⚠ that safety is CONDITIONAL and pinned by a characterisation test — it holds
only because nothing lets an `InCustody` row outlive live custody. ⛔ a durable
save that writes the ledger straight to disk breaks exactly that, and the
annihilation becomes real.

⭐⭐ **MAINTAINER DECISION 2026-08-15 — the CHECKPOINT is the reset baseline.**
Death/retry restores the latest committed checkpoint; traversal and unload
preserve current state. ⛔⛔ NOT `KeyItem => survives reset` — a key item
persists because acquiring it committed a checkpoint. ⇒ the owner must
distinguish THREE horizons: current occurrence state · state at the
reset/checkpoint horizon · durable save. Fixture and full text:
[`maintainer-decisions.md`](maintainer-decisions.md) and
[`engine/instance-lifetime-provenance-and-persistence.md`](engine/instance-lifetime-provenance-and-persistence.md).

⛔⛔ do not answer it by teaching the room loader to inspect inventories — that
is another composition census, and the landed slice's achievement is that room
transition never learned items exist. ⛔ no universal instance registry: the
abstraction belongs around the disposition of the authored occurrence, with
storage discovered from this customer.

✔ **SETTLED: the body owns its inventory and capabilities.** Participant
entitlements and possession-transfer policy are separate concerns with different
owners and lifetimes, so `OwnedItems` is a migration/compatibility
representation, not an undecided authority. ⛔ do not re-open it, and do not
start the `OwnedItems` migration ahead of persistent occurrence continuity,
which has the stronger product pressure.

✔ landed meanwhile, needing no product decision: stow and equip-swap left an
object recording `ItemCustody::Held` by a body with an empty hand — a third
state the enum doesn't have, so an authored axe silently ceased to exist through
the menu. Custody is now re-derived from the hand and RESET to `InWorld`.

✔✔ **ANSWERED 2026-08-18 with the registration assertion it asked for.**
`the_production_plugin_registers_the_custody_release` builds
`ItemPickupSimulationPlugin` and asks the SIM SCHEDULE whether
`return_released_items` is registered — poison-verified by deleting the real
registration (the behaviour test stays green since it exercises a hand-built
chain, while the new guard alone goes red). ⛔ the first draft of the guard was
itself the bug it hunts: it initialized the schedule against a FRESH `World`,
enumerated zero systems, and reported "not registered" for a system that is. It
now asserts a non-empty floor first, so "the enumeration is broken" can never be
read as "the registration is missing" — every count needs a zero floor.

⛔ **Bevy-crate extraction is a criterion applied at every step, never a
follow-up cleanup campaign.** Reach a coherent internal `Plugin` with owned
components/resources/messages/system sets and no upward registration; extract to
a workspace crate when dependency isolation is genuinely real; call it
independently consumable only after a small external-style `App` uses it with no
Ambition content or policy. Never carve because a file or crate is large — a
failure mode this repository has already named twice.

- ▢ **D72 — Continue Smash as a body-generic combat customer.**

⭐ **START AT [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md)**
(2026-08-19): a one-line-per-row table of what the platform-fighter vocabulary
has and what it does not, plus an ordered roadmap. ⛔ grep a row before working
it — the first pass of that table filed tech, ledge-jump getup and the short hop
as MISSING when all three ship.

⭐⭐ **THE TARGET IS SMASH-*LIKE* (Jon, 2026-08-20)**: *"Reproducing smash 4 or
brawl, or melee (bugs are not required parity) would be nice too."* Where the
games AGREE a gap is research — ship the standard. Where they DIFFER the answer
is a KNOB, never a pick; a knob's default is the behaviour that already existed,
so shipping one moves nothing. First worked example is
`MovementTuning::parry_timing`.

⚠ **roadmap item 7 is the 2026-08-20 review's residue** — stale-move accounting
counting CONTACTS instead of USES, `BodyStaleMoves` sitting in the movement core
under the ENGINE rollback domain, and the shared evade control still presenting
as **Dash** on a body whose `AbilitySet::dash` is off. The third is control-surface
shaped and belongs to whoever owns `action_scheme.rs`, not to this row.

Use [`smash-body-generic-combat-2026-08-09.md`](smash-body-generic-combat-2026-08-09.md)
and [`demos/super-smash-siblings.md`](demos/super-smash-siblings.md). The old
migration diary is archived; only source-confirmed residuals should generate
work.

Super Smash Siblings may eventually become a first-class game, but **Ambition
remains the flagship**. Keep both on shared body/combat/participant/world
semantics rather than adding Smash-only engine paths.

⭐⭐ **RECONCILED 2026-08-17 — this row's own rule was vindicated seven times in
one campaign**: *"keep both on shared body/combat/participant/world semantics
rather than adding Smash-only engine paths."* Every defect the smash work
surfaced lived on the SHARED floor, not in smash:

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

⇒ not one was a smash-only path, and not one was fixed by adding one — smash is
the first customer that seats fourteen bodies, runs them CPU-vs-CPU, and
launches them, so it reads floors Ambition's own play had never leaned on.

⚠ the residuals this campaign leaves are named and small: D158's speech-bubble
stacking (closed 2026-08-17 — the offsets were separated and the LINES were
not), and one thing that turned out NOT to be a residual at all on inspection.

⛔ **"presentation hitstop is slot-0 only" was filed as a defect and is actually
a DESIGN FORK.** D114 already fixed the part that matters — both movement roads
spend hitlag now, so both bodies stop on a connect; slot-0's clock-request only
adds freezing everything else (particles, VFX, other bodies) as a flourish.
Slot-0 is correct for what `emit_player_time_intent_system` is for — its other
arms (bullet-time, blink-hold) are per-PLAYER feel affordances (ADR 0010/0011),
so a second player would emit its own intent against its own clock. In a
CPU-vs-CPU match there is no `PrimaryPlayer` at all, and this file already
carries Jon's 2026-08-07 freeze from exactly that shape (a paused match forced
the clock to zero with nobody to ask for the neutral pace back, running the
world at scale 0.0 forever). ⇒ "whose hitstop owns the SCREEN when nobody is
playing" is a real question with several defensible answers (nobody's, the most
recent hit's, the framed fighter's), not a bug with an obvious fix. ▢ now asked,
as §15 in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) —
it had sat as "recorded as a fork" in this row alone since 2026-08-17. ⛔ do not
guess it.

✔✔ **THE SELF-KO CAUSE IS FIXED (2026-08-15) — architecture, not tuning.** The
measured defect (depth 12 survives 7.4s while depth 0 survives 47.8s, a duelist
losing three stocks to itself at 0%) is answered by giving `probe_recovery` its
first consumer: `RecoveryLens` lowers the perceived view into a real `ae::World`
and the body's own `AbilitySet`/`MovementTuning` into a scratch body, and
`refine_by_rollout` now lets the kernel overrule the shadow both ways. ⭐ the
missing half was the REPRIEVE, not the condemnation — the shadow line `Hold`s
after `commit_ticks`, so near a ledge it condemned every verb, and the veto
emptied, falling through to `least_bad_movement` (picks *dies latest*, not
*lives*).

✔ deleted in the same slice: `WorldView::reachable` and `SolidKind::blocks_path`
— hand-rolled straight-line reachability with zero production consumers.

⭐ the falsifier holds: two bodies at an identical position with identical
geometry, gravity and unspent air-jump count, differing only in
`AbilitySet::double_jump`, reach opposite verdicts; poisoning the production path
to a default `AbilitySet` reddens exactly that test and no other — the verdict
comes from the body's capabilities, not the stage's shape.

✔ **THE LEDGE TRANSITION IS CLOSED (2026-08-15) — one predicate.** The shadow's
walk-off test compared the body's CENTRE to `ground_span` while
`surface_supports_body_at_rest` compares its FOOTPRINT, so `left_the_ground`
captured a position a half-extent early (11px on the shipped fighter) at which
the real kernel still found the body standing. Fixed: `integrate` now calls
`ae::collision_semantics::spans_overlap_for_support`, the 1-D core extracted out
of `perpendicular_overlap`, so the shadow asks the kernel's own question with the
kernel's own `EDGE_OVERLAP_SLOP`; the centre test is deleted.
`Perceived::supporting_floor` carries the identical correction from 2026-07-31 —
this was the last centre-in-span ground test in the fighter's ledge reasoning.

⚠ two limits still recorded: blast margins are probed at zero while the stage
authors 120px (does not interact with the ledge case); cost is unmeasured, and
the existing budget bench doesn't price the lens. Probe frequency is unchanged —
still ≤1 per modelled verb per decision.

⚠ **SUPERSEDED 2026-08-15: the tuning pause is lifted, replaced by D128.** The
old note said "do not tune the fighter further until higher-leverage
architecture work is exhausted" — the maintainer has since opened the
combat-expression lane deliberately as a product-pressure slice. ⛔ its licence
is still not tuning: D128 authors repertoire and adds reusable engine
primitives; a dial-turning pass remains out of scope. The deterministic
evaluation rig and its measurements stay. The S4 diagnosis is recorded in
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

⭐⭐ **AND A CARVE LANDED AND WAS MEASURED — `d6ac394ff`, GGRS ROLLBACK HOSTING
OUT OF THE GENERIC RUNTIME (merged 2026-08-19).** This is the row's own thesis
producing a number rather than an argument:

```text
ambition_platformer2d_rollback_ggrs  [NEW CRATE]  +6,440 lines
ambition_platformer2d_runtime                     −5,221 lines
largest_unit_seconds   ambition_platformer2d_runtime 253.3s
                    →  ambition_content              209.1s      ⭐ −44s
```

⭐ **the dearest single compile unit is no longer the runtime**, which is the
payoff a carve is supposed to buy and the one this ledger can see directly.
`edit_cost_seconds` fell ~76s for all three watched crates in the same window,
even though the workspace GREW by a crate and ~1,000 lines — because the lines
that moved left an expensive-per-line crate for a leaf.

⚠ **+6,440 out against −5,221 in is not bookkeeping error**: the difference is
new material the carve wrote (`reconcile.rs`, `registrar.rs`, `registration.rs`,
`codec_tests.rs`, `host_invariant_tests.rs`). ⛔ so this is NOT the debt-laundering
shape this row already paid for once — the destination joined in the same commit,
which is the test that distinguishes a carve from a move.

⚠ the new crate is UNPRICED (median placeholder, R² = 0.12), so its SECONDS
figure is a guess; the LINE movement above is exact and is what the claim rests
on.

⭐⭐ **THAT ACCOUNTING IS DONE — 2026-08-19 — AND THREE OF THE SEVEN
"REGRESSIONS" ARE THE WORKSPACE GROWING, SEEN FROM A FOUNDATION CRATE.**
`edit_cost_lines` is the total lines of a crate's REVERSE-DEPENDENCY CLOSURE, so
for a crate 49 of 59 crates depend on, it is very nearly a measurement of the
workspace:

```text
workspace                       456,072 → 535,388   +79,316  (+17.4%), 56 → 59 crates

ambition_geometry     edit cost 433,774 → 511,209   +77,435  = 97.6% of that growth
                      own lines                       +197
                      share of workspace  95.1% → 95.5%   (+0.4 pts)

platformer2d_core     edit cost 430,276 → 508,302   +78,026  = 98.4% of that growth
                      own lines                     +5,885
                      share of workspace  94.3% → 94.9%   (+0.6 pts)

actor_monolith        edit cost 251,085 → 285,581   +34,496
                      share of workspace  55.1% → 53.3%   (−1.7 pts)  ⭐ IMPROVED
```

⇒ **`ambition_geometry` grew 197 lines and its edit cost grew 77,435.** Nothing
about that crate got worse; the code behind it got bigger. ⛔ **so a carve is the
wrong response to those two findings and always would have been** — carving
geometry cannot move a number that is ~95% workspace size. The only levers are
reducing FAN-IN (fewer crates depending on the foundation) or not growing, and
neither is what "REGRESSED, something got bigger" suggests.

⭐ **the monolith's edit-cost SHARE fell 1.7 points**, which is this row's actual
thesis holding even while its absolute line count is over budget: the
decomposition is working and the per-crate absolute ratchet cannot see it.

⚠ **the accounting above uses LINE COUNTS, which are exact, and is therefore
independent of the ratchet's `UNPRICED` finding** — three crates
(`ambition_binding`, `ambition_boss_encounter`, `ambition_conversation`) are
priced at the population median because nothing has measured them, and size
predicts a crate's compile cost with R² = 0.12, so every SECONDS figure involving
them is off by an unknown factor. ⛔ measuring them means
`compile_collect.py` building the whole graph into its own target root, and this
volume sat at 93% with 36 GB free; that is the operation that filled it before.
⇒ **the guess is accepted for now, and no conclusion here rests on it.**

⛔⛔ **THE INSTRUMENT ASKS A PROXY QUESTION for foundation crates.** Absolute
`edit_cost_lines` conflates *"this crate became expensive to edit"* with *"this
workspace became bigger"*, and for a crate whose closure is 95% of the workspace
it measures only the second. `--report-only` now prints the share alongside the
absolute number so the distinction is visible without changing what the gate
fails on. ⇒ **re-freezing is still not the answer to the monolith's +5,887**;
what these three findings needed was to be read correctly, and now they can be.

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

✔✔✔ **THE CARVE LANDED 2026-08-17 — `crates/ambition_boss_encounter`.** 7,635
lines left the monolith; a second relocation of `features/ecs/bosses/` that an
earlier reading of the 201 inward sites thought was needed first was never
needed — inward sites are callers naming the domain, not a blocker.

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

⚠ **three caveats before anyone acts on this table.** (1) "No sibling depends on
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

⭐ **THE RULE:** when a new target-specific need appears, enable it in the crate
that DECLARES the dependency, and let the app forward a semantic capability. A
future consumer of an Ambition runtime crate should be able to ask for browser
support without knowing what that crate depends on.

✔ verified at HEAD: `ambition_platformer2d_runtime` — which owns `bevy_ggrs` —
declares `web_platform = ["bevy_ggrs/wasm-bindgen"]`, and the app's `web` and
`web_served_assets` personas forward into it. The wrong-half fix
(`getrandom_02` declared at the app) is deleted.

⚠ **`getrandom_03` / `getrandom_04` still sit at the app and that is CORRECT** —
their owners publish no forwarding feature, so the app IS their nearest owner.

Case file: [`../archive/planning-superseded/2026-08-14/d120-platform-capability-placement.md`](../archive/planning-superseded/2026-08-14/d120-platform-capability-placement.md).

- ⏸ **D119 — CLOSED 2026-08-14. The archived-work recovery is done: every item
  archived mid-flight on 2026-08-13 had already been closed by a DIFFERENT
  campaign deleting the thing it was waiting on.** ⭐ nobody re-read the item
  after the road it depended on disappeared — this ledger's oldest failure
  mode; its standing rule is *grep for the thing a row says is missing before
  working it.* Measurement record:
  [`../archive/planning-superseded/2026-08-14/d119-archive-recovery.md`](../archive/planning-superseded/2026-08-14/d119-archive-recovery.md).

  ⚠ **two things survive, both Jon's, neither blocking:**

  1. Three of the run's goal checks in `.goal/active.json` grepped files
     deleted in `5e382342d` — `grep` on a missing path exits 2, `!` inverts it,
     the check reports satisfied. ✔ the goal PREAMBLE was rewritten 2026-08-14
     to scope all of `docs/planning` and route by document ROLE rather than
     filename. ⛔ **the checks themselves were deliberately NOT edited by the
     agent they judge** — quietly rewriting your own success criteria is not a
     repair.
  2. ✔ **DECIDED 2026-08-14: `WornCharacter` STAYS. The `CharacterIdentity`
     rename is REJECTED.** `WornCharacter(CharacterId)` answers *which authored
     character template does this body currently instantiate?* — not *which
     unique runtime occurrence is this?* (`SimId` answers that; D125 makes the
     distinction rigorous). It must stay legal for two bodies to hold
     `WornCharacter(Fia)` at once, and for `RecharacterizeBody` to change the
     worn character while the runtime occurrence stays the same.
     ⚠ if the "worn" metaphor is ever disliked, **`CharacterForm` or
     `CharacterTemplateRef`** preserve the distinction; `CharacterIdentity` is
     specifically the name to avoid.

- ⏸ **D64 — Mary-O / LDtk authoring. RESTING as a successful ACCEPTANCE
  BASELINE, not a running campaign (2026-08-15).** A new level can be created
  through LDtk without adding ordinary Rust level registration: authored rooms
  need no Rust routing to exist, destinations and warp tubes are authored, one
  shared `ldtk_entity_contract.json` makes the Rust prover and the Python
  validator refuse exactly what the real converter refuses, and a ratchet
  guards the level roster. That is an Engine 1.0 milestone.
  ⛔ **do not keep adding Mary-O tooling because the lane existed.** The next
  LDtk improvement must come from actual content-authoring friction.
  Preserved rules: `.ldtk` is the authoritative spatial source · tools edit it
  additively and in place · destructive bootstrap regeneration must not return
  · Rust and Python validation must agree · game-specific semantics stay
  provider-owned rather than growing a central engine taxonomy.
  ⛔⛔ **a row was filed as unstarted when it had already landed**, written from
  a `▢` in Jon's observations file without grepping HEAD. **A marker in a
  maintainer's file is a REPORT, not a measurement.**
  ⛔ **the Mary-O presentation guards do not run in the ordinary suite**
  (`#![cfg(feature = "visible")]`: 36 tests bare, 44 with it). Run the
  `visible` suite before and after any Mary-O visual work.
  ⚠ **the hole is the whole workspace's, measured 2026-08-14**: 24 crates hide
  **629 tests** behind features with no automatic runner —
  `.github/workflows/test.yml` is `workflow_dispatch` only and the per-turn
  gate is one integration target, both deliberate. What runs is a MAINTAINER
  decision; ⛔ do not enlarge `gate_suite.py`, and ⛔ do not add a job to a
  workflow that does not run and call the hole closed.

---

## Waiting on an external fact or maintainer decision

These are real unresolved items but are deliberately **not** `▢` queue work.
⭐ **a `✔` row here is one Jon has since answered**, kept in place for one pass so
anyone who came looking for the question finds the ruling instead of a gap.

- **D23 — projectile collision feel:** authored hurt geometry versus coarse body
  box; see [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §26.
- **D50 — dropped held-item lifetime:** room-scoped versus persistent-world
  semantics; see the decision inbox.
- **D53 — Android suspend/resume:** validate the residual behavior on a real
  device before opening another source fix.
- **D54 — reported visual/VFX issue:** needs the requested reproduction.
- **D70 — Mary-O restart observation:** current tested paths do not reproduce it;
  needs game/room/time context.
- **D42 / D47 — character sizing/rig art:** currently principally authored
  rig/body-inset and visual-review work unless a reproduced engine defect appears.
- ✔✔ **D114 — CLOSED 2026-08-17 BY MAINTAINER RULING. Hitlag freezes the BODY
  that is in it, on both roads, and the old per-body-zero-dt prohibition is
  SUPERSEDED.** `818218949` gave the actor road
  `let sim_dt = if combat.is_in_hitlag() { 0.0 } else { dt };`, so a hit between
  two actors freezes both — which it never did, and CPU-versus-CPU froze
  nobody. Jon, overruling the prohibition: *"hitlag is a combat/body semantic,
  not something that should depend on whether a body happens to occupy the
  primary local-control road."* ⛔⛔ **the three options this row used to offer
  are void — every one preserved a per-road distinction, and the distinction
  WAS the defect. If hitlag ever feels too sticky, tune its DURATION or SHAPE;
  restoring a controlled-body/actor asymmetry is forbidden.** Ruling in
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
generalised:

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

⭐⭐ **ACTIVE TRUTH, checked against `7e7552c4b` on 2026-08-17 — everything below
is evidence, and several of its older claims are no longer true:**

```text
M0  ✔ complete
M1  ✔ met for BOTH halves — conditions AND commands have a domain-owned
       provider contract (PublishCondition / PublishCommand on App, private
       catalogs, no central enum edited to add a provider)
M2  ◐ the PREPARED CALL landed: PreparedCondition / PreparedCommand, private
       fields, no public constructor, id+arity+kind validated at prepare time,
       authored text NOT retained, an authored reference prepared into SimId.
       ⛔ the generic `when … then` container was DESIGNED AND CUT on purpose —
       zero adopters. It needs a REAL CUSTOMER before it is built.
M5  ▢ diagnostics, untouched
```

⇒ two follow-ups are named and are NOT this row's next action: `gated_lock_walls`
still rebuilds its condition arguments every tick instead of holding a
`PreparedCondition`, and `ambition_conversation::dialog::authored_commands`
still owns a second text→`AuthoredArg` conversion that `prepared::prepare_args`
now generalises.

⭐⭐ **M2 acceptance is met for ONE call, not for a `when … then` rule,
deliberately.** `authored_logic::prepared` turns one authored line
(`<id> <arg>…`) into a `PreparedCondition`/`PreparedCommand` with no public
constructor: validation cannot be skipped, the runtime parses nothing, the data
is immutable, and a reference is minted as a real `SimId`
(`SimId::encounter`/`SimId::placement`) rather than a spelling.

⭐ deletions that made this a slice rather than an addition: `KERNEL_FACES`'s
switch→signal pairing and the `SwitchActivated` loop (kernel switches now
author `on_activate: encounter.signal encounter:symmetry_attunement
gravity_down`, performed by the `encounter.signal` command provider); Yarn's
hand-written `YarnStateMirror` bridge (`flag(id)`, `inventory_has(item)`, both
mirror slices, the per-frame inventory refill, a duplicate item-spelling
normaliser, `legacy_dialog_alias`); and `yarn_vocabulary`'s
`cmd_set_flag`/`cmd_clear_flag` (replaced by
`<<command "world.set_flag" "<id>" true>>`). Authored content now asks with
`condition("domain.question", <arg>)` and tells with
`command("world.set_flag", …)` — a domain publishing either adds nothing to any
bridge or vocabulary table. ⚠ the spawn-side half of `KERNEL_FACES` survives
with cause: it builds the encounter's own `Objective::All`, a puzzle stating
its own win condition rather than a table saying which switch does what.

⛔⛔ **the belief that forced the Yarn mirror was FALSE**: three module headers
said Yarn functions cannot be Bevy systems and so cannot reach `&World`;
measured, `bevy_yarnspinner` runs the interpreter from an exclusive system and
threads `&mut World` down to `YarnFn::call_with_world`, and
`SystemId<In<P>, O>` implements `YarnFn`. ⚠ two honest limits stand: a Yarn
*function* call's arity must match exactly (commands, dispatched by name with a
parameter list, do not have this limit), and every authored argument arrives as
TEXT, parsed against the published descriptor's kind.

⭐⭐ **the rollback trick both catalogs share, which is why either could be
waived from rollback at all:** `publish` is PRIVATE on both catalogs, reachable
only through `PublishCondition`/`PublishCommand` on `App`, and a simulation
tick holds a `World`, never an `App` — so "immutable once the simulation
starts" is a property of the TYPE. The command half additionally answers
AUTHORITY (`run` is private; the only public road in is the
`RunAuthoredCommand` message, read by one system) and introduces no new kind of
write (the runner writes into the existing, already-rollback-cleared
`SetFlagRequested` channel). Wire format 358 → 359 stable names, schema v35 →
v36.

⛔ **the named deletion gate (`KERNEL_FACES`'s pairing half) was refused with
cause until its two prerequisites existed — both since PAID (2026-08-17), see
the M2 block above**: a second command (`encounter.signal`) and an authored
LDtk surface carrying a command WITH its arguments (M2's job). ⚠ the condition
half hit the same wall and narrowed rather than invented: `LockWall.gated_by`
names a flag, not a whole condition, because "an authored surface is much
harder to take back than to widen."

Plan: [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md).
Maintainer-identified capability gap: **authoring is strong for nouns —
characters, items, rooms, encounters, sprites, music, platforms, portals,
capabilities — and weak for verbs and relationships over time**: *"when two
switches are active, power a lift"*, *"when an item is placed here, open a
gate"*, *"latch this once true"*, *"wait for a semantic event, then act"* all
currently fall through into bespoke Rust.

Doctrine: **Rust extends the engine's vocabulary; authored gameplay content
composes vocabulary that already exists.** The deterministic simulation still
determines what is true — authored rules invoke explicit semantic domain
operations and never mutate arbitrary ECS state.

⛔ this row does NOT demote D125 or the first capability-aware reachability
customer — D125 is what makes a condition like "item occurrence X is held by
body Y" answerable at all.

⛔ **NOT authorized:** a `UniversalRuleVM`, Lua/Rhai, arbitrary ECS reflection, a
universal scene graph, a central `EngineEffect` enum, or replacing any existing
encounter/cutscene/boss/moveset representation. The several partial
condition → effect systems already in tree are evidence and candidate
customers, not defects.

⇒ **M0 findings (14 systems inspected 2026-08-15), in order of consequence:**

1. ⛔ the shared substrate does not own a universal SEQUENCER, and existing
   domain sequencers (`EncounterScript`'s monotonic cursor,
   `tick_gate_portal_phase`'s reversible timer, `BossPatternState`'s subroutine
   stack) are not to be forcibly unified — the substrate is conditions +
   commands + prepared references + preparation + discovery. A reusable
   control-flow backend stays an optional later experiment, not a current
   priority.
2. ⭐ the gap was on the CONDITION side — no shared condition/predicate type
   existed anywhere; the effect side already had 5+ typed command buses (a
   monolithic `GameplayEffect` enum was built and already deleted — "no god
   enum" is a repeated experiment, not a taste).
3. ⛔ boss patterns are the TEMPLATE, not a customer — they already ship
   authored `.ron`, compile-time cross-ref resolution, a design validator, and
   a cursor that snapshots the resolved timeline. Copy them; do not migrate
   them.
4. ⛔ M4 (is a program counter rollback state?) is not deferrable — three
   shipped answers already exist (register cursor+program, register nothing
   and rebuild, or waive it); whichever the shared form picks changes ≥2
   shipped systems.
5. **moving-platform gating was REJECTED** as a customer — the plan's own
   headline example, and it has nothing to delete (pure addition).
6. no BT/behavior-tree crate is in the lockfile; not refuted, no longer near
   front.

⭐ doctrine correction: a central *authoritative* census every new domain must
edit is bad; a derived *read-only* discovery index that domains contribute
descriptors to is good and required. ⛔⛔ do not sacrifice discoverability in the
name of avoiding central authority. Recorded in
`simulation-authority-and-determinism.md` and
`inspection-diagnostics-and-workbench.md`.

- ✔✔ **the gate portal's rollback waiver was a REAL DESYNC, fixed 2026-08-15.**
  The input rewound and its integral did not: the switch is
  rollback-registered, the phase is that switch integrated over time and was
  not, so a rewind left the integrator permanently ahead — and the consumer
  refuses a room crossing, so it is authoritative, not cosmetic. Fixed by
  deleting `GatePortalConfig.phase` and registering a `GatePortalPhases`
  resource with a value projection (a presence-only probe would have passed
  while reproducing the defect). Schema 31. ⭐ generalisable shape: "a
  registered input with an unregistered integral."
- ✔ **the `Brain` cursor's `_` arm — MOSTLY DISSOLVED.** Six brain families
  looked like they failed to rewind; `rollback_component_cursor`
  clone-snapshots the whole component, so they rewind fine and what they lack
  is desync DETECTION.

- ✔✔ **D133 — CLOSED 2026-08-16, promoted to a prerequisite 2026-08-17, and
  CLOSED AGAIN 2026-08-19 for the case that reopened it. The durable save
  horizon: what the world remembers about occurrences now survives closing the
  program.**

  The on-disk form IS the checkpoint's own description, serialized:
  `AmbitionGameSaveData` gained three `#[serde(default)]` lists
  (`AuthoredOccurrences`, `CustodyBaseline`, `MintedItemBaseline`, save version
  3 → 4) field for field from the checkpoint slice. A load is a checkpoint
  RESUME — adopt the ledger and baselines, write one `ResetToCheckpoint`; the
  road a death already takes rebuilds the world. ⛔⛔ a fixture-found defect: a
  session builds its start room before any file is read, so
  `record_placed_ground_items` republished the stale position over the loaded
  row. Fixed by an INVARIANT — an occurrence comes to rest here only if its row
  says `InCustody` or already `Placed` here, because an object cannot change
  rooms without being carried. Schema 33 → 34 (rename only); the save file is
  not rollback state.

  ⛔⛔ **PROMOTED TO A PREREQUISITE 2026-08-17, then RE-MEASURED 2026-08-19
  against HEAD because the recorded cause was wrong**: a runtime mint not in a
  hand at save time (lying in a room, in flight) was undescribed and lost, for
  THREE independent reasons, each alone sufficient — `live_minted_descriptions`
  filtered out anything not in custody; restore enumerated `CustodyBaseline`
  rows, keyed by custodian, so a dropped item had none to enumerate; and the
  materializer passed `Vec2::ZERO` on the belief no position was needed (false
  — `OccurrenceWhereabouts::Placed { room, at }` already records it).

  ✔✔ **CLOSED 2026-08-19 — a dropped runtime mint comes back where it fell,
  falsifier green and poison-verified**:
  `death_restores_the_checkpoint::a_mint_banked_where_it_fell_comes_back_where_it_fell`
  mints, picks up, drops, banks, dies, and asserts the object is back at the
  position it fell. Fixed across all three causes — `live_minted_descriptions`
  no longer filters on custody; `OccurrenceContinuity` now carries the
  checkpoint's `MintedItemBaseline` beside the ledger, and the reinstatement's
  unsettled-debt arm builds a `GroundItem` request at the ledger's own
  position. The fix cost no wire format — `MintedItemDescription`'s shape did
  not change, only which mints get one. ⭐ the trick was to die WITHOUT
  LEAVING: the death road writes `ResetToCheckpoint` and the room is torn down
  and rebuilt around the body, forcing the same rebuild a round trip would.

  ▢ **what is left is a HARNESS GAP, not a blocker**: a death does not return
  the player to the checkpoint's room (so there is nowhere to look from
  without a hand traveling with the body), and `walk_to` cannot come back
  through a loading zone (the return crossing never fires in 90 frames). The
  falsifier exists and is `#[ignore]`d with both blockers written on it. ⚠ a
  mint out of the INVENTORY resolves through `held_spec_by_id` (the item
  catalog), not `ambition_characters::brain::held_item_by_id` (the brain
  registry) — the narrow lookup answered `None` and lost it a second time.

  ⚠ Jon's dropped-weapon ruling is the product requirement behind this: a
  unique weapon stays where it fell. Also still open: `Consumed` round-trips
  and still has no live producer.

  ✔ **the headless-persistence residue is CLOSED 2026-08-19** —
  `PersistenceSchedulePlugin` was installed only by
  `AmbitionGamePresentationPlugin` ("visible binary only"), so an RL episode or
  a headless test could reach a checkpoint and never write a file. The durable
  horizon is SIM state, so the sim composition now installs the writer, paired
  with `PersistenceRoot::isolated()` (not optional — the default root is the
  player's real platform save dir, and a windowless host already redirects
  audio the same way).

- ✔✔ **D132 — CLOSED 2026-08-19. The same item had two persistence authorities
  and they had never been asked to agree.** ⭐⭐ measured first, and the
  prediction was wrong about which history breaks: the save/load/mint/bank/die
  scenario ended with the player holding AND owning it, the count decremented
  at no beat — that was coincidence, not the defect.
  ⛔ the actual defect: `OwnedItems` was not checkpoint state at all. A pressed
  pickup `grant`ed a catalog row beside taking custody, so one acquisition left
  TWO records and only the object's rewound — acquire after the checkpoint,
  die, and the menu equips a phantom whose throw mints a SECOND real weapon
  (measured: one javelin thrown twice via `<<give_item>>` produced two
  objects, `slot:0/0` and `slot:0/1` — player-triggerable duplication).
  ⭐ closed by DELETION, both halves poison-falsified: the `grant` is gone (the
  object is the record) and `OwnedItems::count` PROJECTS the equipped slot.
  `OwnedItemsBaseline` joined the three baselines a commit already writes
  (protocol 39 → 40), and the mint now spends the row it came from — spending
  without the baseline would be annihilation (a death would retract the minted
  instance while the quantity is already gone).
  ⛔⛔ three recorded answers in `two_persistence_authorities_for_one_item` had
  to be rewritten because they were measurements of the defect — e.g. the save
  reading `1` and the hand holding `1` "agreeing" was the bug (one javelin
  described twice); it now reads `0` and `1`.

  ✔✔ **THE OWNERSHIP QUESTION IS RULED, 2026-08-17.** Jon, verbatim: *"eventually
  we are going to switch to a Morrowind style inventory, so the occurrence is
  the owner, but inventory likely isn't a count it's a set with a count. I
  suppose it will depend on it the item is unique or not."* Correcting his own
  wording: *"and when I say set with count I mean dict… ie each item if has a
  count. for most items it will be a count of 1. note this could also be a
  collection of structs. whatever datastruct makes sense. I'm a python guy not
  a rust guy."*

```text
world       an item is an OCCURRENCE with identity   (held, dropped, placed)
inventory   an ENTRY carrying a COUNT                and the count is usually 1
```

  ⭐⭐ the shape is uniform: an entry is `(item, count)`, most counts are 1,
  twenty arrows are ONE entry with count 20 — uniqueness decides whether two
  entries may MERGE, not how either is stored. ⛔⛔ "dict" is PYTHON VOCABULARY
  for the shape, not a mandate for `HashMap` — the Rust representation is the
  implementer's call.

  ⇒ the rule to design is the CROSSING: a pickup merges into an entry or adds
  one, a drop MINTS an occurrence, and a unique item's identity survives the
  round trip — the same "minted instance not in a hand" case D133 covers.
  ▢ **the one genuinely open sub-question**: what makes two entries MERGEABLE —
  an authored uniqueness flag on the definition, or emergent distinguishing
  state (enchanted, named, partly spent). ⛔ do not answer it by inference.
  ⛔ do not build the general set-with-count today — "eventually" and "likely"
  are his words: the direction is settled, the schedule and exact stacking
  rule are not.
  `a_granted_quantity_survives_the_death_that_retracts_the_instance_minted_from_it`
  is the poison against retracting the row at the reset instead.
  ⭐ the seam, measured on D125: 5 of 6 catalog classes are counts forever; the
  problem is the nine held weapons/abilities that are an instance and a count
  at once. The answer is NOT a row per object — it is deciding which authority
  owns those nine and making the other DERIVE.

- ✔ **D134 — CLOSED 2026-08-16. The workspace-policy suite was 12 violations
  red and nothing ran it; it is 34/34 green.** The twelve were four different
  things wearing one label: one HAZARD (a `HashMap` fixed anyway, so ordering
  is the TYPE's property), two REAL rule violations (`Option<&MotionModel>`
  plus a second the rule could not spell), two POLICY IMPRECISIONS (a
  pre-spawn seed and an off-sim scratch — neither is an entity, so there is no
  authority to route through), one contract that had OUTLIVED its subject, and
  seven that were one wrong fact stated twice.
  ⭐⭐ `runtime → ldtk` was never an upward dependency — `cargo tree` settles it
  in one line: the ldtk crate's transitive closure contains zero occurrences of
  the runtime or the monolith. Both rules changed, with the argument written
  into their own `rationale` fields. ⚠ `bevy_ecs_ldtk` stays denied in both —
  the runtime may compose the adapter, never the backend it exists to contain
  (poison-tested red).
  ⭐ the durable lesson, on its THIRD instance in this file: deleting a
  compatibility facade reddens a boundary policy, every time, and that policy
  is the second file nobody remembers to edit.
  ⚠ one blind spot left standing deliberately: the movement rule matches
  SPELLINGS, and `Option<&ae::MotionModel>` escapes it. Recorded in that
  policy's rationale rather than widened into a crate this slice did not
  analyse.

- ✔ **D137 — CLOSED 2026-08-17 (`f7e34225d`). The doc-link ratchet was RED and
  is GREEN — no crate has risen and `--check` exits 0 for the first time since
  the row opened.**
  ⛔⛔ **`--check` IS LOAD-BEARING, measured by poisoning the baseline**: the
  bare command prints "ROSE from 5" and **exits 0**; only `--check` exits 1. A
  step wired with the obvious invocation would have been a gate that CANNOT
  FAIL.
  ⛔⛔ **a false "it is missing" claim was published here.** The ratchet had
  been in CI all along (`format-and-clippy`, `ccf254ff2`); a duplicate step was
  shipped then removed. The tell was in the output: the grep printed the
  workflow file TWICE and both lines were attributed to the step just written.
  ⇒ check the file as it was BEFORE the edit (`git show <commit>~1:<path>`).
  What was genuinely missing and is now wired: `cargo test -p
  ambition_workspace_policy` in `engine-tests`.
  ⛔⛔ **a CARVE LAUNDERS DEBT off any ledger keyed by crate name, and the
  ledger congratulates you for it.** The conversation carve took the monolith
  122 → 109 and invited banking it — those thirteen were RE-HOMED into a crate
  the list did not name. Adding `ambition_conversation` put 11 still-broken
  links back on the books, moving the honest total 182 → **193**. ⇒ the rule
  written into `CRATES`: **the destination joins in the same commit.** ⛔ never
  run `--update` to clear someone else's rise.
  ⭐ per-turn gating is answered NO by measurement — 51 s warm, **338 s after
  touching one crate** — so the gate runs pre-push or in CI. ⚠ the cost grows
  with every carve.
  ⚠ **the residual debt is NOT repaired** (193 broken links): fixing what a
  session broke is not paying it down. ⛔ do not bulk-delete the brackets —
  that converts a detectable break into an undetectable stale sentence. The
  stake, in the ratchet's own words: *"a deletion that leaves its references
  behind turns a doc comment into a description of a world that no longer
  exists — which in this repository is where the reasoning lives."*

✔✔ **D166's FIRST THREE SLICES LANDED 2026-08-19, in the 2026-08-19 GPT
review's own order.**

```text
1  action LEGALITY vs UTILITY   `ActionLegality` filters the option list; a
   (the review's item 1)        press the body cannot consume is not offered.
                               capture_probe: 54 presses/33 wasted -> 9/0,
                               holds 1 -> 2.
2  what a HOLD is worth         `capture_value` prices a capture from the
                               OPPONENT — guard, percent, hitstun, airborne —
                               never from a constant. The reverted
                               throw-damage pricing is pinned by a test.
3  capture RELATION vs POLICY   pummels/hold-age/escape left `CapturedBy` for
   (the review's item 3)        `SmashHoldState`; protocol 38 -> 39.
4  George SELECTS, not owns     his facet moved to the character-authoring
   (the review's item 2)        submodule; the demo's pack points at it.
5  every fighter has a grab     12 of 14 had `capture: None`; now none do.
```

▢▢ **THE CARVE IS COSTED — measured 2026-08-19, the boundary is now a
CHECKABLE CLAIM.** `brain/fighter` is 10,644 lines; the GENERIC brain named it
in five places (widened from an initial undercount of three — the contract's
`fighter::` grep under `brain/` couldn't see a rollback codec that hand-writes
`StateMachineCfg::Fighter` by field, or `brain/mod.rs`'s string mapping):

```text
1  brain/snapshot.rs          `attack_kit: Vec<fighter::options::AttackCandidate>`
                              — every brain pays for a field one of them reads
2  brain/state_machine/mod.rs `StateMachineCfg::Fighter { cfg, state }` — the
                              shared brain cannot compile without the fighter
                              one; needs a registration seam
3  brain/smash/emit.rs        ✔ CLOSED 2026-08-19 — `TILT_DEFLECTION` moved to
                              `crate::actor::attack_gesture`
4  brain/snapshot_impls.rs    `Brain`'s rollback cursor codec hand-writes the
                              per-variant tag and fields
5  brain/mod.rs                maps the variant to the string "fighter"
```

⭐ `ambition_platformer2d_core` does NOT depend on the fighter brain (its one
`fighter::` hit is a doc reference) — the one edge that would have made the
carve impossible does not exist.

⭐⭐ **the seam has a proven shape already in this tree**: `SmashHoldState`
(2026-08-19) solved the same problem one domain over — platform-fighter state
that used to ride inside a generic thing now rides BESIDE it as the
capability's own component, registered through `RollbackRegistrar` by the
capability's own domain. Applied here: `Brain::StateMachine(StateMachineCfg::
Fighter { cfg, state })` (a closed enum) becomes `Brain::Capability(BrainId)`
plus `FighterCfg`/`FighterState` as the capability's own components. That
single change removes all five edges. ⚠ the cost is a dispatch that finds a
registered brain rather than matching an arm.

⭐⭐ edges 1 (`attack_kit`) and 2 collapse to ONE: `attack_kit` is read only by
`tick_fighter`, called only from the `StateMachineCfg::Fighter` arm, so it
travels with the fighter brain the moment that variant stops being a variant
of a shared enum. ⇒ **one blocker remains: the brain-registration seam.**

⇒ the ratchet is `the-generic-brain-does-not-grow-new-platform-fighter-edges`
(`scripts/check_absence_contracts.py`), pinning the (now four) remaining edges
and refusing a fifth. Order: `smash/emit.rs` (done), then `snapshot.rs`, then
the state-machine variant that needs the seam.

⛔⛔ **the review's prohibition is NOT yet satisfied and is deliberately still
open**: *"I would specifically avoid adding things like `Features {
grab_value: ... }` to the generic scorer. That just moves the smell into a
struct."* A `capture_value` feature WAS added to `brain/fighter`'s `Features`
and works — but `brain/fighter` is not yet a platform-fighter CRATE, so the
point stands and is what the carve has to relocate. The review withdraws its
earlier hold on carving: *"I no longer think 'do not carve yet' should be
treated as indefinitely binding… the carve has now been earned."*

⚠ one number in that review is superseded: it says the CPU spent 36.5% of a
match in grab range "without one free-body Grab press" — true on the tree it
read; as of `15a2c1b8e` the CPU throws grabs from free bodies and lands holds,
pummels and throws. Its architectural conclusion is unaffected.

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

⭐⭐ **AND THE OPEN HALF WAS RE-MEASURED, 2026-08-18.** `capture_probe -- 60` with
no `--force`, George versus Alice, found the grab landing from a free body ZERO
times in 3600 ticks despite 1313 ticks (36.5% of the match) inside its own 42px
range: `REACH_TOLERANCE = 2.0`, so `reach_fit(44, 110) = 1 − 66/88 = 0.25` — the
grab survives the "cannot reach" filter at 2.5× its own reach while every shorter
move is filtered to zero, so **at long range the grab is the last option standing
and wins by default**, and the body is at long range precisely BECAUSE it just
threw a smash and is in its recovery. ⇒ **the grab is chosen exactly when it
cannot be pressed, and never when it could work** — at close range the jab and
the smashes beat it, because they carry damage and it carries none.

✔✔ **BOTH HALVES WERE THEN BUILT AND MEASURED, 2026-08-19 — the numbers above are
superseded, the causal story above them is not:**

```text
                        the run above   +policy   +airborne   +legality
grab presses                        7        85          54           9
  wasted while committed            6        34          33           0
grabs started                       1        22          14           9
holds established                   0         1           1           2
```

⇒ `capture_value` prices a hold from the OPPONENT — guard, percent, hitstun,
airborne — rather than from damage the move does not deal; `ActionLegality` stops
the brain offering a move the body cannot begin. ⛔⛔ **`REACH_TOLERANCE` was
neither widened nor narrowed.** The spacing corrected itself once the grab had a
reason to be thrown at the right moment instead of being the last option standing
at long range — which is this row's own thesis confirmed, not a tuning change.

⇒ so the missing term is not "grabs are worth their throw's damage" (tried and
reverted, above) and not a wider tolerance. It is that a grab's worth in this
genre is **that it beats a shield and leads to a throw** — which is
platform-fighter policy, exactly what this row says the capability should own,
now with a match to point at instead of an argument.

⭐⭐ **AND THE SAME MEASUREMENT FOUND A COMBAT INPUT BUFFER THAT IS DESIGNED,
ROLLBACK-REGISTERED, AND WIRED TO NOTHING.** Checking `trigger_moveset_moves` and
`ResolvedAttackGesture` alone found no latch and looked like absence — wrong,
because `AxisManeuverState`'s own field doc said the design outright:

```text
"Buffered MOVEMENT actions (jump/burst/blink press windows). Combat
 buffers (attack/pogo/projectile) stay on the shared BodyActionBuffer."
```

⇒ so the movement half is REAL (`buffer_jump`, `buffer_burst`, `buffer_blink`,
`coyote_timer`, all inside the rollback-registered `MotionModel`), and the combat
half is designed but unused:

```text
BodyActionBuffer { attack, pogo, projectile }   on every actor
rollback                                         registered `body.action_buffer`,
                                                 CANONICAL codec — it costs
                                                 schema and snapshot bytes
production reads/writes of its FIELDS            0
BodyActionBuffer::tick callers                   0
```

⇒ **a press in the last frames of recovery still does nothing**, for a person as
much as for a CPU. ⭐ the rollback question is ALREADY ANSWERED by precedent —
`MotionModel` carries the jump buffer through the same schema.

⚠ still a maintainer's call, because it changes the feel of every character in
every game the engine runs — the same shape as the `REACH_TOLERANCE` question
above, and possibly the same conversation. ⚠ `body.action_buffer` is currently a
row in the rollback schema for state nothing produces: implement it or retire it,
but a canonical-codec component with zero writers is paying rent.

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

⭐⭐ **A SIXTH INSTANCE, 2026-08-19 — and it is the row's failure mode in its
purest form: a boundary drawn from a real hazard by whoever installed the thing
first.** `PersistenceSchedulePlugin`'s own doc said *"for visible builds.
Headless / RL drivers omit this plugin so they never read or write user files."*

```text
the hazard, real     writing the PLAYER's files
the line drawn       persist only in visible builds
what that cost       an RL episode, a fuzz run or a headless test could reach a
                     checkpoint and never write one — the durable horizon is SIM
                     state, so this was a capability that existed only when
                     somebody was watching
```

⇒ **the two were conflated, and separating them is the whole fix**: any
composition that SIMULATES installs it, and a non-player App owes its own
`PersistenceRoot` — `isolated()`, the same redirection a windowless host already
makes for audio. Both halves are now written on the plugin itself, where the
next person looks, and asserted in one test because installing one without the
other is worse than neither.
⚠ **the shape to recognise**: the sentence naming the hazard was CORRECT and the
sentence drawing the line was not, in the same doc comment. A stated boundary is
only as good as the question it answers — *"who may write the player's files"*
is answerable; *"which builds persist"* was a proxy for it.

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
      (CLOSED — see D131, below)
D134  `runtime → ldtk` was forbidden by two policies nothing ran; the EDGE turned
      out legitimate and downward — but only because a facade deletion converted
      a laundered edge into a declared one, for the THIRD time in that file
D135  the canonical session world carries an authoring-format-specific field, and
      five RON-only games construct `::default()` for a world they never install
      (CLOSED — see D135, below)
```

⇒ **the through-line: none of these is a bug in the ordinary sense.** Each is a
place where *"who is this for?"* was answered by whoever installed it first, and
never written down. ⭐ that is why the composition-shaped slices have been paying
more than the feature-shaped ones this week — naming the schedule order let
`conversation` leave, giving the engine a home for its own art made effects
reachable, and making a load a checkpoint resume removed a whole reconstruction
road.

⚠ **D131 sharpened the umbrella and cost it a member.** Its composition failure
was real and was **not a rule reaching a foreign body** — it was an authored
NUMBER (`max_health`) read by another game's rules, so nothing a system-scoping
mechanism could have caught. ⇒ **the umbrella has two shapes, not one**: (a) a
value authored under game A's rules read as universal by game B, and (b) a
global SINGLETON whose owner is whoever installed it last. D131 also MEASURED an
instance of (b) and left it standing on purpose (later fixed — see "death
rules", below). ⛔ two shapes measured is the argument for a scoping mechanism;
one was not, and D131 deliberately did not build one.

⚠ **the standing number to move**: `capability-footprint-may-not-grow` reads
**42 crates linked, 15 a movement-only game never asked for**. ⇒ **a slice that
claims this row should say what it did to that number, or say why the number is
dominated by something it did not touch.**

⚠ **D135 (CLOSED — see below) was the first executable instance, and it answered
the standing number above with a NO**: the footprint did not move, because
`ambition_platformer2d_ldtk` was held in a movement-only game's closure by the
MONOLITH — seven production files needing nine symbols (`WorldManifest`,
`LdtkProject`, `ActiveLdtkProject`, `LdtkHotReloadState`, `poll_ldtk_file_changes`,
the `field_*` readers) — and not by the session world at all. ⇒ the next instance
to take was a monolith carve at the world-manifest/asset-catalog seam.

⇒ **THE WORLD-MANIFEST INSTANCE LANDED 2026-08-16, AND THE COUNTER STILL READ
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

✔✔ **RELOCATION OF THE LDTK HOLDERS IS EXHAUSTED, 2026-08-16 → 2026-08-18 —
production refs to `ambition_platformer2d_ldtk` in the monolith went 5 → 0.**
`WorldManifest` and the hot-reload watcher moved out (not LDtk-shaped); the
remaining five files were genuinely LDtk vocabulary
(`LdtkProject`/`LdtkLevel`/`ActiveLdtkProject`/`field_*`), so the cure was
INVERSION onto the room IR rather than further relocation. `EncounterTrigger` and
`LockWall` — markers the room-IR converter had deliberately dropped as "read by
their own consumers off the raw `LdtkProject`" — now emit through `RoomSpec`, so
`load_encounter_specs_from_rooms`, `authored_gated_lock_walls` and
`authored_switch_commands` all read `&RoomSpec` instead of the project.
`SwitchCommandSpec` and the lock-wall fields ride as their own typed family
alongside the CLOSED Tier-0 `PlacementSchema` rather than folding into it, so no
fingerprint/replay schema event was owed. Verified live (boot census unchanged at
`1 encounter entit(ies)`) and poison-verified at each step (the converter
emitting nothing turns the relevant loader/lock-wall/switch tests red); the
change signal moved with the data too (`ActiveLdtkProject::is_changed()` →
watching the room set), because a reload that rebuilds rooms under an unchanged
room id would otherwise serve stale derived state forever.

⚠ **one surviving production edge is a different KIND of thing, not a missed
inversion**: `assets/loading.rs` declares
`Handle<bevy_ecs_ldtk::assets::LdtkProject>` to load the file — an asset-loading
declaration that the app loads the file, not a consumer reading world facts off a
project instead of the room IR. Whether an asset collection in the monolith
should name the format at all is a separate boundary question.

✔ **the dep is `optional` now** in the monolith's manifest, taken back only
through `[dev-dependencies]` for tests.

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
  RON-authored games filled it with `::default()` for a world they never
  install.** Fixed: the field became `Option<…>`, private, `None` by default,
  installed only by the LDtk road — taking with it a `demo_fixture` re-export,
  three setup systems, and six systems that had rebuilt against an empty index
  every tick in five games (now `.run_if(ldtk_world_installed)`). Guarded by two
  tests: a RON game has no LDtk index, asserted only beside a positive check that
  the LDtk-authored game installs a real, non-empty index — the negative alone
  would pass vacuously for a broken implementation. ⛔ an absence or population
  count taken with a repo-wide grep must exclude worktree clones (a grep swept
  `.claude/worktrees/`, inflating this row's own site counts). ⛔⛔
  `capability-footprint-may-not-grow` did not move (still 42 crates linked, 15
  unwanted) — dominated by the MONOLITH, not the session world; the next slice
  is a monolith carve at the world-manifest/asset-catalog seam (carried by
  D136, above).

- ✔ **DEATH RULES STOPPED BEING A PROCESS-GLOBAL `Resource` (2026-08-16)** —
  three games each inserted one in `Plugin::build` and the last one won, so
  every Smash match in the shipped host ran under Mary-O's death rules. Fixed by
  routing through `mode_scope`, the existing mechanism that already scopes a
  hosted game's systems and entities to its own rooms: a game declares into
  `DeclaredDeathRules` under the rooms it governs, `governing(mode)` answers
  "whose rules apply here?" through one `SystemParam`, an unclaimed room reads
  `LevelReset::Never`, and a second declaration over one scope panics at build
  rather than picking a winner. ⛔ `ExperienceScopeBuilder` does not fit this: it
  releases state on route departure with no entering half, so `DeathRules` would
  be deleted forever on the first departure. `sync_hosted_sanic_wallet_shield`
  (⚠ GONE — consolidated at `03d4c8d22`) was the same bug in miniature —  a
  system whose population was every `PrimaryPlayer` in the process rather than a
  global — and got the same fix.

- ☑ **D131 — CLOSED 2026-08-16. Four fighters were being divided by 1, 1, 60 and
  100.** `damage_percent()` is `accumulated / max`, and `max` was each
  character's HOME GAME's authored pool — Mary-O and Sanic author
  `max_health: 1` because they are one-hit-kill platformer protagonists, so one
  point of ordinary damage read as **4200%**. Fixed: the MATCH declares what
  100% means (`MatchRules::pool_over(authored)`, applied at both seat sites),
  deleting the 2026-07-31 per-character workaround that stamped a reference onto
  only the three ids that demo registers. ⛔⛔ the swap-to-P2 control pointed at
  the wrong cause: it proved the cause travels with the CHARACTER, but what
  travels is the authored VITALS, not a system — the crossover-plugin hypothesis
  was falsified in the same run (zero deaths, zero replays). What crossed the
  boundary was a NUMBER, not a rule.

- ☑ **D130 — CLOSED 2026-08-16 BY LOOKING. (a) There was no tofu — it was the
  STAGE FLOOR** (tiles and blurred parallax HUD chips, photographed at 3x with
  bevelled highlights and dark borders); it read as tofu because
  `--route smash_gameplay` with no roster puts the camera at its default
  position with no subject, so the floor sits alone with no scale cue. The HUD
  font fallback was innocent. **(b) FIXED** — `capture_scene` grew a step that
  carries a POSITION: `--press touch:XxY` sends the pair of real `TouchInput`
  messages winit emits, driving the same phone road the product ships. Cause:
  key taps are edges with no position, and the tool's bare `Enter` fired
  wherever the cursor sat. Guarded by
  `the_capture_tools_documented_taps_seat_two_cpus_on_two_fighters`.

- ▢ **D129 — The sprite pipeline CUTS ART AT THE LOGICAL FRAME AND NOTHING NOTICES.
  (opened 2026-08-16 from a maintainer observation, measured the same day)**

Jon: *"Super sanics spikes are clipped by the sprite renderer. This might need a
structural fix. We should not be able to clip sprite artwork so easily."*
⇒ **true, and it is not one character.**

✔✔ **GUARD LANDED 2026-08-16** (renderer `6228c58`) — the renderer now WARNS,
at draw time, when a frame's drawing runs off the logical frame, naming the
animation, frame and edges. It warns rather than raises because 52 sheets
already trip it and a fatal check would block regenerating anything until the
whole roster is redrawn; whoever fixes the art can make it fatal. Seven tests.

⛔⛔ **the sheet count is a BUILD-TIME observation, not a property of the
repository** — `clipped_frame_edges` runs from `sheet_build.py:943` on the
drawing canvas BEFORE padding, and the shipped PNG no longer carries the
evidence because the packer already trimmed it. Re-measuring means a full
regen of all 196 sheets (gitignored art), so `52 of 196` is a SNAPSHOT dated
2026-08-16/17 — anyone quoting it later should say so. ⭐ the practical
consequence: this row closes by REDRAWING, and the measurement comes free
with the redraw — the guard fires on the next build of any sheet somebody
touches.

⛔⛔ **the criterion that finds real cuts is: a truncated shape does not
TAPER** — compare the edge line to the widest the shape reaches a few lines
in; a tip narrows on its way out, a cut arrives already near full width.
Denominator-free, unlike the two criteria tried first: *"touches a
logical-frame boundary"* flagged 74 of 133 uselessly (with `auto_crop` the
frame is fitted to the art, so touching is normal), and this row's original
*"≥6 opaque px in a run covering >25% of the edge"* hid an unchosen
denominator and made the original **"23 sheets"** count untrustworthy.
Verified against the live alternative explanation (auto_crop merely hugging
the art) by the opaque-width profile of the topmost rows — a real tip tapers
from nothing, a cut starts wide:

```text
super_sanic  idle     top rows ->  12 14 17 18 20 22 24 25   ⛔ no tip: CUT
sanic        idle     top rows ->   0  0  0  0  3  6  8 11   ✔ a taper
sanic        jump     top rows ->   0  0  0  0  0  0  0  0   ✔ touches, NOT cut
player_robot_v3 idle  top rows ->   0  7  9 11 11 13 13 13   ✔ and off.y=73
```

⚠ **the taper test is right about walking poses and WRONG about a
bottom-anchored resting pose** — flat shoe soles arrive at the boundary at
full width and never taper, so an idle can read as cut and not be. The
discriminator for that case is drawing the same painter into a TALLER canvas
and checking whether ink actually appears past the boundary;
`bottom_center_canvas` is a plain paste, not an ink re-anchor, so nothing
downstream can put lost pixels back.

**Re-measured with the taper criterion: 52 of 196 sheets, with frame counts.**
Worst first — `ninja_shadow_oni_leader` 73 frames (all four edges),
`ninja_shadow_duelist` 70 (all four), `player_combat_review` 108,
`player_traversal_review` 100, `trex_enemy` 57 (bottom+left), `super_sanic` 54
(top — Jon's report, now fixed, see below), `raid_enforcer` 52,
`fascist_enforcer` 53, `pulse_voyager_captain` 48,
`perfect_cellular_automaton` 45, `goblin_shaman_staff` 39,
`tech_bro_disruptor` and `goblin_cantina_chieftain` 35, `robot` 34,
`robot_guardian` 33, `m_leblanc` 32, `player_extended` 30, and 35 more with
fewer frames each (`pirate_admiral` only 2, `oiler_vfx` 1). The control that
makes it causal: base `sanic` is clean and only `super_sanic` (same body,
`spikes_up=True`) is cut.

⭐ **23 sheets collapse to far fewer causes by source YAML** — eight
(`robot`, `player_extended`, both player `*_review` sheets,
`robot_caster`/`diver`/`miner`/`runner`) are auto-emitted from
`robot_spritesheet.yaml`; the two `sandbag_*_review` from
`sandbag_spritesheet.yaml`; `ninja_shadow_oni_leader` and
`ninja_shadow_duelist` share `ninja_spritesheet.yaml` — ~15 authoring sources
total, the robot family the largest (cut edge `top`, the antenna spike in
`player_robot_v3.py`; `robot_spritesheet.png` is embedded in
`ambition_asset_manager`, so this ships). The current player draws
`player_robot_v3`, the CLEANEST sheet measured (margin 20) — name which sheet
a character actually draws before calling any of this player-visible.

✔✔ **super_sanic — the sheet Jon reported — FIXED 2026-08-18** (renderer
`39d79a7`): the raised fan and back blade are scaled by a named
`SUPER_SPIKE_FIT = 0.76`, the largest value measured that leaves every frame
whole (swept 1.00→61/181 cut, 0.85→4, 0.80→2, 0.76→0), poison-verified
against base `sanic` (0 of 181) so a later change that shrinks every spike or
grows the frame cannot pass. ⛔ **"just make the frame bigger" is refused
with cause**: `auto_crop` is deliberately OFF for this sheet so
`ATTACK_HITBOXES` coordinates match draw space — growing the frame or
shifting the body would silently move every authored hitbox. For a sheet
with authored hitboxes in draw space, the ART is the only thing that can
give — worth checking before proposing a canvas change on any of the other
~15 authoring sources.

✔✔ **Mary-O's walk-frame clipping FIXED 2026-08-18** (renderer `a17b8bf`):
every walk frame put her foot up to a fifth of a tile below her own standing
line (`+dy` is down; the trailing leg's `leg_back_dy=+1.0` at toe-off pushed
the foot through the line instead of lifting off it; the passing pose's
`bob=+0.4` was the same mistake at a third the size) — a leg-reach sign bug,
not a framing one. Fixed to `+0.00` on both forms; 7 frames left the
clipping guard's list (`mary_o_v2` 14→13, `mary_o_v2_tall` 11→8,
`mary_o_v2_fire` 6→3); the three canonical images are byte-identical
through it. ⭐ the guard asserts against her OWN idle, not the frame height —
her standing line moves with per-form scale, so "below the frame" would
have been a proxy for "below the floor." Poison-verified.

▢ **what is still cut on Mary-O, and it is NOT a pose** — measured
2026-08-18: three numbers that should be one, at 6px/authored-unit on a
192px published frame:

```text
                    drawn sole    authored collision_bottom_px    foot socket
small (one brick)     194 px               190 px                   176 px
grown (two bricks)    190 px               192 px                   176 px
```

⇒ the small form's sole is 2px below its own frame (the sliver still cut on
every frame of that sheet) and 4px below its collision box; the grown form's
sole is 2px above its box; and both forms' `foot_r`/`foot_l` sockets are the
same hardcoded `output_px(88.0)`, 14–18px above where either foot actually
is. ⛔ **not fixed here on purpose** — every repair moves where she STANDS,
and that is D165's call (Jon, by eye: "small Mary-O is one brick, grown is
two"); the measurement was the missing part, which of the three numbers is
authority is his, and the sockets are the one no form currently derives.
`grow`/`shrink` (both sheets) and `death#0` (top) are separate frames with
their own reasons.

✔✔ **the renderer's own suite was RED and is now 1 failed / 620 passed**
(triaged 2026-08-18, ten of eleven were bad tests, not code — read what a
check computes before believing what it reports):

```text
✔  test_no_raw_imagedraw      found a REAL defect en route: the boot thruster's
                               nozzle ellipse replaced the bloom's alpha instead
                               of compositing (61 of 396 player frames lost glow)
✔  test_svg_parts_cache  x2   message named a dependency (resvg_py) that WAS
                               installed — `_native_resvg_callable` requires
                               `inspect.isbuiltin` by design, the tests' pure-Python
                               fake was refused, and the fallback (CairoSVG,
                               absent) raised a message about resvg instead
✔  test_actor_contract        a dead exemption plus a proxy question
✔  test_character_notes  x2   written for a schema that changed under them
✔  test_robot_slash_hitboxes x2  froze one build of the art as the requirement
✔  test_geometry_gui          held a reference the drag handler REPLACES
✔  test_portrait_product      froze which clips a boss draws from
✔  test_every_registered_character_target_has_local_actor_metadata  its exemption
                               was dead (rig-doc targets carry metadata fine) and
                               its question was a proxy (module constant vs a
                               function); the fix's first draft guessed the
                               attribute name and returned `{}`, staying red for a
                               new reason that looked like the old one
✔✔ test_rig_codegen_and_scale THE ONE REAL BUG — see below
```

✔✔ **the one real bug, CLOSED 2026-08-18** (renderer `6162a4a`): a target
generated from a rig document rendered a different image from the document
it came from. `RigDocument.render_at` downsamples through
`resize_transparent_sprite(reducing_gap=3.0)` specifically because a Lanczos
kernel's negative lobes leave a pale halo on a silhouette over transparency;
`rigdoc_codegen` still emitted `img.resize(..., LANCZOS)` — the exact call
the interpreter had deliberately stopped making. Alpha difference across the
body was 15 against a tolerance of 2; both roads now take the same
downsample path including the `SS == 1` short-circuit. Every character
published from a rig document had been shipping the halo the interpreter
removed.

⛔ **do not bulk-fix a red suite** — each failure is a different question,
and two are "is the assertion or the art right", a look-at-it call. One
remaining failure (Mary-O's visual baseline) stands against an uncommitted
side-view strap edit in the worktree — it belongs to whoever lands that art,
in the same commit.

▢ **and one survivor is genuine, left red on purpose:**
`test_generated_matches_rigdoc_render` compares the rig document's own
renderer against the module generated from it, and they disagree on every
clip:

```text
alpha delta       max 11, >2 on 355 px, >8 on 8 px   (the tolerance is 2)
visible delta     max 19.8/255 on 137 px, mean 0.32
alpha bbox        rigdoc 39x78 vs codegen 43x82 — a 2px fringe all round
NOT a translation — every 1px shift makes the match WORSE
```

⇒ composited, the two pictures are indistinguishable; the disagreement is a
sub-pixel rasterization difference, and what introduced it is unknown. ⛔ do
not "fix" it by re-basing the assertion on a visible-difference metric —
that would hide that two roads meant to emit one picture no longer do.

⛔ **why nothing caught this at all**: the drawing canvas IS the logical
frame, so overflow is clipped at draw time before anything downstream can
see it — the packer's post-trim losslessness check compares trim geometry
against the logical frame and cannot see ink that was never drawn. The guard
has to be at draw time, in the renderer, stating the invariant once over
discovery rather than once per sheet.

✔✔ **the fix has two separable halves, and (b) is RULED 2026-08-17: CASE BY
CASE, DRIVEN BY THE WARNING.** (a) the draw-time check [landed above]; (b)
the sheets that are already clipped — a sheet is fixed when its clipping is
actually visible in play, and whether that's by growing the logical frame or
re-authoring the art is a per-sheet call. ⛔ **do not open a bulk campaign
and do not bulk-grow frames** — the population stays known-bad by design,
driven sheet-by-sheet by the warning.

⛔⛔ **this orders against `BodySource::SpriteAuthored` migration**: the
renderer's measure-by-default principle ([`engine/sprite-renderer.md`](engine/sprite-renderer.md))
reads a clipped sheet's geometry faithfully off art that was already cut, so
migrating a clipped body to `BodySource::SpriteAuthored` would bake the cut
into its collision box. Fix the clipping first for any body in both lists.

- ◐ **D128 — Can this engine carry a serious platform fighter through ORDINARY authoring? (product-pressure vertical slice, opened 2026-08-15; ⭐⭐ EVERY ENGINEERING LINE IS ✔ AS OF 2026-08-18 — what is left is Jon playing one match)**

⭐⭐ **the executable list below is EMPTY.** Pacing was ruled, respawn
placement and asset composition landed, CPU symmetry landed, and all four
presentation defects are closed — 5 and 6 fixed and photographed, 7 was
fixed three hours after it was reported, 8 verified live. ⛔ **so do not
open this row looking for work**; it stays ◐ only because its QUESTION is a
product one and the answer is a person watching a match, not another
capture.

✔✔ **PACING IS ACCEPTED — RULED 2026-08-17.** Jon, verbatim: *"A three-stock
CPU showcase finishing in under ~40 seconds is certainly not too long; if
anything it is brisk. **Do not retune stock count, knockback, or damage**
around that partial 20-second frame. … Human-vs-human balance can be judged
separately later."* ⭐ he also corrected how the row read its own capture —
it led with a 1200-tick frame (180%/124%, nobody dead) and buried the fact
that at 2400 ticks (~40s) the match had COMPLETED and returned to CHOOSE
YOUR FIGHTER. ⇒ **when a capture sweeps time, the acceptance question is
answered by the LAST frame, not the most alarming one.**

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

⚠ **"another capture is done" was true of the ACCEPTANCE question and NOT of
the defects.** Captures on 2026-08-18 confirmed defect 5 live, verified
defect 8's fix end to end, and — once the warmup landed on **360**, the tick
the report actually names — caught defect 6 in the act; two earlier warmups
(300, 420) settled nothing. ⭐ the documented tap recipe still works: nine
taps seat two CPUs and start a match unchanged.

✔✔ **CPU SYMMETRY: TWO CPUs WEARING ONE CHARACTER WERE THE SAME MIND, AND
THAT IS FIXED (2026-08-17).** The fighter brain seeded its noise stream from
difficulty alone (`0x5F37_7A11 * (level+1)`), so any two CPUs on one rung
drew byte-identical noise and mirrored each other exactly on a symmetric
stage — every other template in `brain_builders.rs` already varied off
`seed_from_id(&enemy.id)`; the fighter was the sole outlier. Fix: seed from
`PreparedSeat::feature_id` (`"<character>#seat<n>"`), distinct per
participant rather than shared by every twin; no clock, no process-global
RNG, no Bevy `Entity`, so replay determinism holds
(`the_same_participant_rebuilds_on_the_same_stream`). ⛔⛔ needed a SECOND
site: `project_authored_fighter_ladder` was rebuilding `FighterState` with
the same level-only constant on `Added<Brain>`, so it now CARRIES
`state.noise` across the rebuild.

⭐⭐ **Emmy Ethereal now AUTHORS the old (shared-stream) behaviour as her own
trait**, not inherited from the bug — `CharacterDefinition::preserves_mirror_symmetry`
(`.preserving_mirror_symmetry()` in `authored/npc_emmy_noether.rs`) drops the
participant term and keys on the character, so only her twins share a
stream. It does not zero the seed and touches nothing else; it synchronises
NOTHING per tick (the mirror breaks correctly once observations diverge —
`the_same_seed_shown_a_different_world_may_decide_differently` is the
falsifier). No `if character == Emmy` in the AI: generic code reads a bool
the character authored, riding `ActorConfig` (`rollback_component_clone`).

⭐⭐ **measured on the real stage**: under the old shared stream, two same-character
CPUs stayed an EXACT mirror image (equal-and-opposite about the midline,
identical y, to the float) for a whole match. ⛔⛔ the first draft of that
test was VACUOUS — it measured DISTANCE, which passes with the defect fully
present since two fighters spawned apart drift regardless of their brains.
The metric that answers the question is **MIRROR ERROR**,
`|(x0−mid)+(x1−mid)| + |y0−y1|`. Both the fix and the ladder projection's
carry are poison-checked.

✔✔ **and Emmy is now pinned in the full host too**
(`game/ambition_app/tests/smash_cpu_cognition.rs`, ungated) — the standalone
smash app cannot seat her at all (no `ambition_content`), so this is the
first place *"the character a player can actually pick off the grid gets the
shared stream"* is asserted. ⭐ measured in the full host, rung 5, two CPUs
on one character:

```text
                     streams     mirrored for       match ran
npc_emmy_noether     IDENTICAL   2576 of 2576 fr    2576 fr   a stalemate
npc_pirate_admiral   DIFFERENT    488 of 1548 fr    1548 fr   they fight, and it ends
```

⚠⚠ **488 frames is ~8.1s, and Jon reported it from play**: *"it took a while
for Booule to desync, but they eventually did. And Emmy never desynced.
Still the desync for non-Emmy CPUs probably should happen sooner."*
⛔⛔ **the cause is NOT the seed, and two fixes were built, measured and
reverted**: the stream has exactly ONE consumer (press-timing jitter, only
on a decision that commits to an attack), so a different RNG cannot separate
two bodies doing the same thing. Per-participant decision phase: 488→220fr
but broke five behavioural guards in `the_stage_kills` (a 0-4 tick offset
changed whether attacks connect) — reverted, too high a price. Cadence
drawing from the stream: 220→219fr, nothing — reverted.
⇒ ▢ **what would actually move it is asymmetric CIRCUMSTANCES — per-seat
spawn placement (defect 3, already landed)** — two fighters starting
somewhere different take a genuinely different first decision; it will also
shorten Emmy's mirror, correctly, since the assertion that would notice
says so. Not yet re-measured against the landed spawn fix. ⛔ do not build a
third randomness fix before reading the note at
`brain_builders::fighter_cognition_seed`.

⚠ **the catalog PREVIEW brain still seeds level-only, correctly** — a
preview has no match and therefore no participant; the note at
`character_catalog/resolver.rs` forbids copying that back onto a
construction road.

⭐ **the state acceptance was given against a two-CPU match captured
2026-08-17, AFTER D155 gave the game working knockback** (34%→180% in
thirteen seconds, real exchanges with hit VFX, the stock loop closing on its
own) — every feel judgement recorded anywhere in this row before that date
was made on a build where nobody was ever launched, and is void.

⛔ **the eight named defects from the 2026-08-16 photo session — do not
re-derive their status, do not re-run the capture:**

```text
1 self-KO on every stock  ◐ substantially repaired — the CAUSE was architecture
                            (RecoveryLens, 2026-08-15); the 2026-08-17 ladder
                            re-run reads d0 no self-KO / d12 first at 21.8s.
                            ⛔⛔ the RecoveryPolicy ledge-grab diagnosis this row
                            used to name is RETRACTED and banned — a photograph
                            falsified half the CPU-suicide finding.
2 camera loses the fighter ✔ CLOSED — `frame_the_cast` always framed every live
                            seat; three downstream sites threw it away (room
                            clamp, stable_center, 8 Hz ease). Guarded by
                            `every_live_fighter_stays_inside_the_frame`.
3 both seats respawn at    ✔ CLOSED 2026-08-18 — `respawn_placement` takes the
  ONE overlapping point       SEAT; seats alternate outward from the centre, so
                              the arrangement is symmetric at any roster size
                              and no seat is privileged. Guarded by
                              `every_seat_comes_back_to_its_own_point_on_the_platform`
                              (no two within a body width, symmetric about the
                              centre, every seat still ON the platform). The
                              pre-existing `a_respawn_is_above_the_stage_centre`
                              asserted `respawn.x == centre.x` — the defect
                              stated as an invariant — so that clause was
                              corrected rather than deleted.
4 winner card names a SEAT ✔ CLOSED by D140/D148 — the card reads
                            `WINNER: Robot v3`, a team keeps its team name.
5 barks draw as a          ✔ CLOSED 2026-08-18, photographed both ways. Overlap
  screen-wide caption         half closed via D158→D159 (a bubble is a
                              `WorldLabel` in the one ranked placement pass).
                              Scale half: before 535px of a 1280px frame (41%
                              of a 640-wide stage), after 185px in a centred
                              column (−65%) — the cause was that a bark had NO
                              WIDTH AT ALL (`spawn_speech_bubble` set only
                              `font_size`). ⚠ width bound only, never height:
                              `TextBounds` truncates past its bound on wrap, so
                              a height bound would silently eat a long bark.
6 untextured olive quad    ✔ CLOSED 2026-08-18 — it reproduces, and the fix is
                              that art the engine already ships now gets asked
                              for. Cause: `spawn_impact` draws a bare rectangle
                              BY DESIGN as its no-decoded-sheets fallback — not
                              a failure (`note_effect_miss` logged nothing).
                              The art (`generic_action_fx`: hit_soft/hit_hard/
                              hit_metal/hit_energy) was on disk the whole time;
                              `spawn_hit_marker` now draws `hit_soft` at 0.9x
                              `FX_DEFAULT_WORLD_SIZE`. ⛔⛔ 2026-08-18's first
                              verdict ("not reproduced") was an INSTRUMENT
                              ARTIFACT: the scan flagged any 40x40 window >95%
                              one colour, and the artifact is 19x19 — a
                              systematic scan is only as fine as its window.
                              ⇒ not taken here: `ImpactMaterial` and the
                              sheet's four hit rows are two vocabularies still
                              unjoined — asked as §18 in
                              [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md),
                              Jon's taste call (`hit_hard` is a strength
                              distinction, not material, so material picks the
                              row explains only 3 of 4 rows).
7 VFX authored against no  ✔ CLOSED 2026-08-18 — was ALREADY FIXED 2026-08-16,
  size reference              three hours after it was reported (`d6d5810b8`,
                              same day as the `39dc7a39b` report): inline
                              `render_size = BVec2::splat(132.0 * scale)` is
                              now `FX_DEFAULT_WORLD_SIZE = 56.0` plus a
                              per-move `Vfx { scale }`. At the measured 1.594x
                              zoom the old 132 units drew at 210px; the new
                              largest VFX in the warmup-360 frame is 107px (56
                              units at the Admiral's authored scale 1.20)
                              around a 46-unit fighter. ⇒ before photographing
                              a defect, `git log` the file it names — an
                              observation older than the commit that fixed it
                              is not evidence about today.
8 capture_scene prints no  ✔ CLOSED 2026-08-18, verified live on a real
  pose for a 2-CPU match      two-CPU match: `seat 0 at (350.4803, 276.0000)` /
                              `seat 1 at (233.9185, 276.0000)`, where it printed
                              NOTHING before. Reports each SEATED body, sorted
                              by SEAT (not query order, which would make two
                              captures of one match differ). When there is
                              neither a primary player nor a seated body it now
                              says `NO SUBJECT … this image proves nothing
                              about a pose` instead of printing nothing.
```

✔ **standalone `game/ambition_demo_smash_app` CLOSED 2026-08-18** — it
composed no asset install at all (no `PlatformerAssetsPlugin`), the only
demo shell that never joined the pattern `ambition_demo_mary_o_app` and
`ambition_demo_sanic_app` already use
(`PlatformerAssetsPlugin::for_experience(SMASH_EXPERIENCE).with_room(...)`
then `PlatformerPresentationPlugin`, AFTER `compose_smash_shell` because the
plugin READS the catalogs it registers). ⚠ Smash reached through the shell
(`ambition_app`) is a DIFFERENT, always-fine composition — say which binary
any claim is about.

⚠⚠ **what is and is not verified**: ✔ 33 headless tests still pass and both
feature configurations compile. ⛔ the plugin's `build()` was NOT executed —
the crate's tests are gated out under `visible`, so `cargo test --features
visible` links and runs ZERO of them; the ordering rests on matching two
working shells and on the plugin's own panic naming the mistake, not on
booting art-less. ▢ **a windowed run is what closes the loop, and it is the
one thing this session could not do.**

⛔ **answered and must not be re-asked**: *"do the two authored kits read as
different fighters?"* — YES, inside four seconds. *"is the VFX/SFX road
reachable?"* — YES (`ebc8877ee`). *"has anybody watched a match?"* — YES,
twice.

⭐⭐ **the outcome is pinned, not just the mechanism** — `the_stage_kills` (17
tests, green) includes `a_second_match_on_the_same_stage_counts_in_and_ends`,
CPU vs CPU, one stock, so the end arrives from a single CPU-produced launch
reaching the blast zone rather than a test writing a velocity. ⇒ the
05:45 capture showing both fighters at 180%/124% with 3/3 stocks was a
mid-match MOMENT, not evidence of a broken KO.

✔ **the capture is done — 2026-08-17, three frames of one two-CPU match**
(George Booul vs Pirate Admiral, documented taps, 1280x720): 420 ticks
34%/36% both fighting; 1200 ticks 180%/124% still nobody dead; 2400 ticks
back at CHOOSE YOUR FIGHTER, every slot NOT PLAYING. The stock loop closes on
its own; percent climbs and fighters engage (34%→180% in 13s with hit VFX) —
first time true since D155. ✔ shown to Jon and accepted 2026-08-17 (see the
pacing quote above).

✔ **the capture found one real presentation defect, closed same-day as
D158→D159**: speech bubbles stacked illegibly (two CPUs taunting at once is
the ordinary case on this stage) — fixed by making a bubble a `WorldLabel` in
the one ranked placement pass, not by retuning the stack offsets.

⛔ **not a defect, checked before reporting**: George Booul renders as a
white ghost — that is his authored art (the select-screen portrait grid
shows the same ghost).

◻ **HISTORY, done twice (2026-08-16, then 2026-08-17 after knockback
worked) — do not run a third capture to establish status; read the ACTIVE
TRUTH above.** The 2026-08-16 capture (21 frames, two matches) found the
documented nine-tap command seats the WRONG pair: `747x121`/`425x121` are
grid cells 3/0 (Sanic and Player Robot v3, the generic-kit floor), not the
authored pair. ⛔⛔ **and the correct cell coordinates have moved AGAIN,
since 2026-08-20** — George Booul is cell 1 (`touch:479x121`) and the Pirate
Admiral cell 4 (`touch:801x121`) now that one appended fighter took the
roster grid from 15 cells to 16 and re-flowed it to six columns. ⚠ quote the
CELL, not the pixels. ▢ **the two literals in `capture_scene`'s header and
in `the_capture_tools_documented_taps_seat_two_cpus_on_two_fighters` still
need updating to the current cells (~15 min)** — otherwise every future look
through the documented command answers the standing "do the kits behave
differently" question with the wrong pair. That 2026-08-16 capture's own
headline: the two authored kits DO read as different fighters, but every
stock in both matches was lost to the void at ≤6% damage — since fixed by
D155's knockback repair; the pacing/self-KO status above is current.

✔✔ **first fighter landed and verified (George Booul, 2026-08-15)** — the
gap was mostly AUTHORED REPERTOIRE, not missing engine: `special_*`,
`Cancelable`/`OnHit`, per-volume `on_hit`, `motion_scale` tails, multiple
Active windows and per-move Sfx/Vfx were all already sufficient and unused.
The one real gap — a move could not displace its owner at a chosen moment,
nor command a speed rather than add to one — became
`MoveFrameData::{lift_speed, lift_at_s}`, now used by the recovery probe as
`RecoveryLift`. ⚠ two gaps named rather than half-built: no per-airtime
budget for an airborne self-impulse move, and `WindowTag::Invuln`/`Armor`
parse but have no consumer.

✔✔ **second fighter landed too (Pirate Admiral, 2026-08-15)** — a
materially different lateral/grapple recovery concept; recovery routes are
now evaluated through the real movement kernel (`movement/recovery.rs`)
rather than ranked by a static "this is the recovery move" property. ⛔ do
not redispatch "make a second fighter".

✔✔ **the CPU lane landed and was measured (2026-08-15)**: distinct attacks
used per match went 5-6 of 16 → 9; all four of George's specials now appear;
the duelist's whole vertical game (air_up/air_down/smash_up/tilt_up) was
absent and is now present. Two causes, both initial hypotheses wrong: (1)
the attack stick wrote a facing-relative `+x` into a field the resolver
already multiplies by facing, so a forward/back attack chosen while facing
left came out reversed AND at full deflection (a FLICK, never a tilt); (2)
the kernel's `.first()` route-search fallback endorsed a recovery in only 3
of 100 `Situation::Recovery` decisions — deleted outright.
✔ Jon's three couch items also fixed: camera close eased (was 237-361 units
in one frame against 33-49/frame open ramp, now 68.9); the match-end race
(Smash despawning the eliminated body while `decide_stocks_match` read sides
off bodies that still existed) fixed by ordering; the countdown was firing
into `GameplayBannerRequested`, which nothing drew — the winner card had the
identical defect with a unit test that only asserted the message.

✔✔ **VFX/SFX road landed 2026-08-16 (`ebc8877ee`): an effect is a NAME, and
the engine ships the art it draws.** Design owned by
[`engine/render-animation-and-vfx-extension.md`](engine/render-animation-and-vfx-extension.md).
`FxId` is the authored row name on the
wire (`SfxId`'s FNV-1a hash, borrowed not re-typed). `ambition_sprite_sheet::fx`
resolves name → (sheet, row, `vfx.<family>.<row>` cue); `GameAssets.fx` is
the engine-owned home. FIVE reconstruction tables deleted (`ExplosionKind`,
`move_vfx_kind`, `explosion_anim`, `explosion_sfx`, plus five
`CharacterAnim::from_name` aliases spelling FX rows as Idle/Walk/Run/Hit/
Slash). The "no world at validation time" constraint that looked like it
would force a hand-kept 189-row table dissolved instead: `build.rs` already
bakes every `*_spritesheet.ron`, so `ambition_sprite_sheet::fx::is_authored_effect`
is a pure, world-free oracle, and `expand` takes it as a parameter rather
than naming the crate. **189 authored rows ↔ 189 cues, one for one, across
all twelve FX sheets, no sheet off by one.** ⚠ none of the underlying art is
in git (gitignored) — a fresh clone needs `./regen_sfx.sh` and
`./regen_sprites.sh` to get it; the roster commit is the durable half.
⛔⛔ this is what exposed the standalone-Smash-app asset-install gap above —
same defect shape one level up.

⚠ **the product question, still standing** (this is what the row's ◐ is
waiting on):

> can someone watch a CPU-vs-CPU match and immediately see several
> mechanically distinct attacks, aerial choices, specials, an intentional
> recovery move, expressive movement and convincing impact — and conclude
> this engine can elegantly support a serious platform fighter?

**Scope:** one existing body, ≥8 materially distinct attacks (rotated clones
do not count), a real authored Up-B in the ordinary moveset architecture, a
launcher, a punish/kill move, a mechanically interesting aerial, authored
SFX/VFX through normal content mechanisms. ⛔⛔ CPU usage is part of
acceptance: the generic policy layer must actually use the repertoire (≥5
distinct offensive move ids, aerials, a special, the authored Up-B for
recovery). ⛔ no character-ID conditionals in AI — derive affordances from
move data (coverage, startup, reach, launch direction, commitment, impulse)
before adding annotations; only a technique whose behaviour cannot be
inferred from static geometry (teleportation) may expose its own affordance.

⛔ **not authorized:** grab/throw architecture · shields/parries as a
subsystem · ledge-rule parity · many characters · balance · animation
redesign · combo scripting · networking · no character-specific system to
compensate for a missing generic mechanic. ⚠ an authored Up-B is the one
sanctioned link to `RecoveryPolicy` reachability (its default presses only
`side ∈ {0,±1}` plus jump) — do not begin the general navigation graph.

⇒ **why this lane runs with the other two** (maintainer's exception): the
combat lane is orthogonal enough to the systemic-world and rollback lanes to
stay independently integrable. ⛔ narrow or pause it the moment it starts
changing the same authority boundary as another live lane.

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
