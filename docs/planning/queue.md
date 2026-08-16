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
established** — one number saying how much world a sheet pixel covers. It exists,
it works, it has two users, and thirty-three bodies are still sized by a constant
somebody eyeballed once. **Snake too big, Sanic too small and the player hurtbox
being wider than the art are three of those thirty-three.** ⇒ the executable
next step is *migrate a body to `SpriteAuthored` and see whether its report
disappears*, starting with whichever of the three has a sheet whose
`body_metrics` look right — not a fourth constant.

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

- ▢ **D125 — The systemic world substrate: what a thing IS, which occurrence it
  is, why it exists, and how long it lasts.**

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

⚠ **the limitation, and it is the durable-save leg:** materialization is bounded
by *"some room authors a record with this id"*. A **runtime-minted** instance —
the throw's `SimId::spawned` arm, an enemy death drop — is room-scoped and
carryable, so it can enter the custody baseline, and **no record anywhere can
rebuild it**. Today it warns and is lost. Closing it needs a durable *instance
description* rather than a pointer at an authored record, which is the same
unclosed leg `ItemCustody`'s own doc names.

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
known opt-out, not the open-ended risk it looked like. ▢ **ready to run.**

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
target. Prefer boundaries that improve capability closure, compile isolation,
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
- ⛔ *void crossing (recorded body gone)* — **MISSING, and it is a FORK.** Terminal
  on the rollback host (`CommitOutcome::Cancelled` drops the intent AND retires the
  transaction); an ordinary retryable failure on the eager host, which leaves the
  intent pending — so a headless eager host reopens and re-fails **forever, with no
  backoff and no report**. One outcome where the confirmed side has three.

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

- ⏸ **D127 — Deterministic authored gameplay logic and orchestration. M0 COMPLETE; M1 PARKED behind D125 and reachability.**

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

✔ **M0 is COMPLETE** (14 systems inspected at HEAD, 2026-08-15). ⛔ **it is no
longer the executable step** — M1 is, and M1 is **parked behind D125 and the
reachability customer**. Nothing here is authorized to start.

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

- ▢ **D129 — The sprite pipeline CUTS ART AT THE LOGICAL FRAME AND NOTHING NOTICES.
  (opened 2026-08-16 from a maintainer observation, measured the same day)**

Jon: *"Super sanics spikes are clipped by the sprite renderer. This might need a
structural fix. We should not be able to clip sprite artwork so easily."*
⇒ **true, and it is not one character.**

**Measured.** Criterion: a frame whose trimmed rect touches a logical-frame
boundary AND has ≥6 opaque pixels in a straight run along that boundary covering
>25% of it — the signature of a flat cut rather than art that merely ends there.

```text
133 sheets scanned
 74  have at least one frame TOUCHING a logical-frame edge
 23  show the FLAT-CUT signature
```

Named: `super_sanic` (top — Jon's report exactly), `robot` (171 frames, top),
`player_extended`, `player_combat_review`, `player_traversal_review`,
`robot_caster` / `robot_diver` / `robot_miner` / `robot_runner` (top),
`puppy_slug` (bottom+left+right), `ninja_shadow_oni_leader` (three edges),
`perfect_cellular_automaton`, `trex_enemy`, `galwah`, `m_leblanc`,
`mantis_lancer`, `ninja_shadow_duelist`, `pulse_voyager_captain`, `trent`,
`goblin_desert_bow`, `ranged_skirmisher`, two `sandbag_*_review` sheets.

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

⭐⭐ **MEASURED 2026-08-16, and it reorders the work — the refusal is the THIRD
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
