---
status: built — measured on the smash stage 2026-08-29; open for feel
owner: handoff written 2026-08-29, picked up and built the same day
---

> **BUILT.** `0dd64feea` (the kernel's wire), `f920e092c` (the move and the
> rope), `6dc8833f2` (`wire_probe`, and three things it caught). All six clauses
> are measured below. What is still OPEN is FEEL: nobody has played it. The
> numbers say the mechanic is there; they cannot say it is fun, and the knobs to
> turn are named in `performer_moveset.rs`.
>
> ✔ **RE-VERIFIED 2026-09-03.** All three commits resolve and say what this page
> says they say; `wire_probe` is still a registered binary
> (`game/ambition_app_tools/src/bin/wire_probe.rs`, 19K, `[[bin]]` at
> `game/ambition_app_tools/Cargo.toml:83`), so the measurement above can be re-taken rather than merely
> cited. ⓘ **And `git log` answers the question this page leaves open:
> `performer_moveset.rs` has not changed since — nobody has turned a knob.** So
> "open for feel" is not a stale label on work that quietly moved on; it is the
> accurate current state, and it is parked exactly where
> [`../decision-principles.md`](../decision-principles.md) says a tuning task
> belongs — shipped blind, waiting on Jon, blocking nothing.

# The Performer's up-B is a WIRE, not a teleport

Handoff for a machine picking this up cold. Read this, then
`game/ambition_content/src/performer_moveset.rs`. Her down-B (the Trap) landed
2026-08-29 and is the worked example for how this one has to be proven —
[the proof bar](#the-proof-bar) is not optional and is most of why the Trap took
three attempts.

## What it measures, on the stage, with presentation live

`cargo run -p ambition_app_tools --bin wire_probe -- right render`

| Clause | Measured |
|---|---|
| 1. Not a teleport | 32 ticks on the wire; largest single tick **16.6px**, against 215px in ONE frame |
| 2. No teleport sound | `player.blink` emitted **0** times across the whole move |
| 3. A wire from the sky | rope drawn on **32 of 32** on-wire ticks, on the ACTOR road |
| 4. She is lifted | rose **431.9px**, monotonically |
| 5. A fairly large distance | **0.90 platform widths**, 1.80× the fall blast depth, 2.01× the teleport |
| 6. She swings | **±90.6px** across at the cut, leaving at **±169 px/s**, an exact mirror |
| Still a recovery | `gates.recovery` spent at t0 |

⭐⭐ **AND THE MIRROR IS THE INTERESTING ONE.** Run `offstage`, which starts her
below the lip and off the side: swinging TOWARD the stage lands her on the boards
by t90, and swinging AWAY drops her past the blast line at 760 px/s. The recovery
is real and it is a DECISION — which is the whole point of clause six.

## ⛔ Three things that measured wrong, and what they cost

Recorded because each is a shape, not an incident:

1. **The handover was a COIN FLIP.** A hard stop at the swing cap (clamp the
   angle, zero `ang_vel`) made a held stick leave at either full tangential speed
   or nothing, depending on which side of a tick she clipped the stop: the kernel
   measured +229 px/s and the probe measured 0 for the SAME wire. The kernel's
   test was asserting the lucky side with a one-sided `> 150`. The stop is soft
   now and both instruments land in one band.
2. **ONE FORMULA, THREE HOMES.** The winch rate that travels the authored rise
   was solved in the executor while the profile was integrated in the kernel, so
   they were free to disagree — and the moment the profile gained an ease-out
   tail they did, silently, undershooting. `winch_rate_for` lives beside the
   profile it inverts; the executor and both suites ask it.
3. **FOUR MEASUREMENTS SAMPLED THE WRONG MOMENT.** Every one was a loop bounded
   by a TICK COUNT rather than by the state it was about, so it read ordinary air
   control twenty-seven ticks after the wire let go and credited it to the swing.
   One of them would have passed for a release that wrote no velocity at all.
   ⭐ **the tell is always the same**: a loop that runs `for _ in 0..N` and reads
   a value at the end.

## What Jon asked for, in his words (2026-08-29)

> *"Now we need to fix her up-b. It is not a teleport and should not get the
> teleport sound. It needs to be a rope or wire that reaches down from the sky
> (it can instantly appear as if it went from visible to invisible), but she
> doesn't teleport up, she gets lifted up by the wire, a fairly large vertical
> distance, and while she is being lifted by the wire her motion controls should
> let her swing like a pendulum so she has a bit of horizontal recovery with it
> too. So, this might need a bit of engine work to make the motion work and be
> expressable elegantly."*

Six clauses, and each is a falsifiable claim about a running match:

1. **Not a teleport.** She travels through the intervening space over time.
2. **No teleport sound.** `player.blink` must not be emitted by this move.
3. **A wire reaches down from the sky.** It may pop into existence — no draw-in
   animation is required.
4. **She is LIFTED.** Continuous upward motion, not a placement.
5. **A fairly large vertical distance.** Larger than the 215px the teleport
   covers today; a recovery you can see happen.
6. **She SWINGS like a pendulum under motion control**, buying *some* horizontal
   recovery — this is the clause the engine does not have a seam for yet.

## What it is today, and why every clause fails

`the_flyline()` in `performer_moveset.rs`:

```rust
let spec = hitless_special("performer_curtain_call", "fly", WIRE_AT_S, WIRE_ENDS_S);
let spec = author_teleport(spec, WIRE_AT_S, TeleportParams {
    behind_nearest_foe: false, behind_gap: 0.0,
    distance: 215.0, ledge_assist: 44.0, intangible_s: 0.12,
    depart_vfx: "four_point_glint".into(), arrive_vfx: "four_point_glint".into(),
});
UpSpecial::Standard(spec).into_spec()
```

`WIRE_AT_S = 0.12`, `WIRE_ENDS_S = 0.46`.

It is **literally the Author's teleport with a different comment**. One beat, one
placement, 215px, and `apply_authored_teleports`
(`abilities/traversal/teleport.rs:470`) emits `PLAYER_BLINK` at every transit —
that is the teleport sound Jon hears, and it comes from the executor, not from
this timeline, so deleting a cue here will not silence it. The `four_point_glint`
on both ends is the Author's blink flash.

⛔ **Do not "fix" this by muting the executor's cue.** A move that runs the
teleport executor IS a teleport; the cue is telling the truth. The move has to
stop running it.

## The engine work

This is the interesting half and the reason the row exists. Read
`docs/concepts/one-body-one-path.md` before choosing a shape.

**What exists to build on:**

| Seam | Where | What it is |
|---|---|---|
| `BodyMode` | `platformer2d_core::player_state` | `Standing`/`Crouching`/`Crawling`/`Sliding`/`MorphBall`/`Climbing`/`Submerged`. What a body IS — its gravity, geometry and hittability. `Submerged` was added for the Trap and is the closest precedent. |
| `MotionModelSpec` | `platformer2d_core::movement::model` | `AxisSwept` / `SurfaceMomentum` / `AdhesiveCrawler`. HOW a body integrates. Swapped with `switch_motion_model`, which preserves private state across an unchanged variant. |
| Authored techniques | `ambition_characters::smash_*` | `smash_teleport`, `smash_trapdoor`, `smash_ride`, `smash_vitality`, `smash_bomb`, `smash_capture`. Each is a **key + params** a moveset authors, plus an executor in `abilities/`. The moveset says WHEN; the engine says WHAT IT MEANS. |

**The shape this most likely wants:** a tether. A body on a wire is not a body
with a velocity — its position is `(anchor, rope length, angle)`, its input is
angular, and the winch shortens the rope. That is a genuinely different
integration, which is what `MotionModelSpec` is for; `BodyMode` is for what a
body IS. A pendulum is still a normal, hittable, drawn body — so the likely
answer is a **new `MotionModelSpec` variant, not a new `BodyMode`** — but that
call belongs to whoever does the work, with the reasoning written down.

⛔ **`Climbing` is not it, and neither is a scripted arc.** Climbing is anchored
to a region it must keep touching and has no angular state. A canned arc has no
motion control, and clause 6 is the whole point.

⚠ **The swing must not become a flight.** `UpSpecial::Standard(spec).into_spec()`
stamps `gates.recovery` (`smash_repertoire.rs:188`) — an up-B that spends nothing
is flight, and D204 already says most up-Bs should be once per airtime. Keep the
slot lowering. Horizontal recovery is *"a bit"*, not a free traversal.

**Presentation:** the wire itself. `rendering/submerged.rs` is the worked example
of a procedural, per-body visual with a lifecycle (spawned while a state holds,
retired when it ends) and carries the reasoning for why it is procedural rather
than an FX-atlas row. ⛔ It also carries the trap this row must not repeat — see
below.

**Audio:** a wire is rope and pulley, not a star flash. `world.door.heavy_open`
was the Trap's answer to the same question. Nothing in this move may reach
`player.blink`.

## The proof bar

⛔⛔ **THE TRAP WAS DECLARED DONE TWICE WHILE VISIBLY BROKEN IN PLAY, and both
times the instrument was the problem rather than the code.** Do not repeat these:

1. **A moveset test proves the SPEC, not the move.** Every authoring test on the
   Trap was green while the move did nothing, because both halves of a
   two-authority bug were individually correct on the spec.
2. **The sim is not the game.** The Trap's simulation was right for weeks while
   presentation never heard about it. Anything a player *sees* has to be
   observed through a host with a render app.
3. **⛔ THERE ARE TWO VISIBILITY ROADS.** `BodyPoseView` is the session's
   exploration player; `FeatureView` is every ACTOR, which is what a Smash
   fighter is. `PlayerVisual` is inserted in **exactly one place in the engine**
   (`session/setup.rs`). A rule stated on one road is not stated. This is the
   bug that made her draw and blink under the stage, and every existing test in
   `submerged/tests.rs` spawned a `PlayerVisual`, so none of them could fail.
4. **⛔ AN AMBITION ROOM IS NOT THE SMASH STAGE.** The Trap's presentation fix
   was verified with `capture_scene pirate_cove player` — the one road where the
   broken gate happens to pass. Jon: *"when we are doing smash moves we probably
   should be using the smash stage and not any ambition stages."*
5. **⛔ TWO AUTHORITIES FOR ONE FACT MEANS ONE OF THEM IS DELETED.**
   `LEAP_OUT_SPEED = 430.0` was authored as an `Impulse` AND as a surfacing beat
   on the same frame; the inline impulse was overwritten by the later system's
   `TransitVelocity::Zero` every time and moved her never. For the wire, the
   things at risk of being written twice are **position** (the winch vs. the
   pendulum) and **exit velocity** (the release vs. whatever places her).

### The instrument

`trap_probe` is the convergence instrument and already does most of this job.
**Extend it or copy it; do not start from a unit test.**

```bash
cargo run -p ambition_app_tools --bin trap_probe -- right render
```

It seats a real `smash_roster(["performer","performer"])` on the smash stage,
drives the production input road (`drive_control_frame`), and prints per tick:
position, velocity, body mode, live hitboxes, door count, and the visibility
chain on **both** roads (`player[views/sub/hidden] actor[views/sub/hidden]`).
Args: `left`/`right`, `hold=N`, `downheld`, `render`, `host=demo`.

Two blockers were cleared to make a rendering probe possible, and both will bite
again if a new binary is written from scratch:

* `VisibleRenderMode::NoWindow` sets `backends: None` and **omits the render app
  entirely** — every presentation number it reports is a zero that means nothing.
  Use `OffscreenGpu` via `build_visible_app_with` plus a `HeadlessDisplaySurface`.
* A hand-stepped `app.update()` **does not wait for the wgpu device** that
  `app.run()` waits for, and panics inside `no_automatic_skin_batching` with
  *"Res\<RenderDevice\> ... Resource does not exist"*. Pump
  `plugins_state()`/`finish()`/`cleanup()` first.

⚠ `capture_scene` renders but **cannot drive a controlled press** — its cast is
CPU-brained and walked off the stage and died during a 300-tick warmup. It is
for looking, not for staging.

### What "proven" means for this row

A run that prints, on the smash stage, with presentation live:

- [ ] the body's **y over time** — a climb across many ticks, not a jump in one
- [ ] the **total rise**, in px, stated against the stage so "fairly large" is a
      number somebody can argue with
- [ ] **horizontal displacement under left/right input**, and the **mirror** of it
      (the Trap's ledge-vs-cap question was settled by running it both ways)
- [ ] **no `player.blink`** anywhere in the move — assert on the emitted cue, not
      on the timeline, because the executor emits it and the timeline does not
- [ ] the **wire visual** present for the lift and retired after it, counted on
      the ACTOR road
- [ ] `gates.recovery` still spent, so it is a recovery and not flight

Plus guards at **both** layers, as the Trap has: an authoring guard that the move
carries one authority for its motion (the shape of
`the_leap_has_one_authority_and_it_is_the_surfacing_beat`), and an executor guard
that the engine actually moves the body (the shape of
`surfacing_with_a_leap_speed_launches_her_out_of_the_boards`). A guard on only
one layer is what let the leap ship deleted.

## Also open, unrelated to the wire

* ⚠ **Her art may still be `actor_*` on your machine.** The generated sheets are
  git-ignored, so the rename travelled in the catalog, `scripts/regen/sprites.sh` and the
  build script but **not on disk**. `character_catalog.ron` names
  `sprites/performer_spritesheet.{png,ron}`. Rename the six files in
  `crates/ambition_platformer2d_actor_monolith/assets/sprites/`
  (`actor_actor.ron`, `actor_portraits.{png,ron}`,
  `actor_spritesheet.{png,ron,yaml}` → `performer_*`, and patch the `target:`,
  `image:`, `character_id:`, `sheet_id:`, `output_stem:`, `renderer_target:` and
  `default_preset:` fields inside them), plus any copies your machine has in
  `sprites_0_5x` / `sprites_0_25x` / `sprites_potato`. Or regenerate — the roster
  is correct now. `declared_art_resolves` says when it is done.
* ⚠ **The Trap's subterranean beat is longer than the stage.** She crosses the
  ~465px of ground in 1.9s against a 3.0s beat, so from mid-stage she spends over
  half of it pinned motionless at a ledge. `MAX_UNDER_S` is the one knob and the
  call is Jon's — this is a tuning question, not a defect.

## Receipts

Landed 2026-08-29, and each is a worked example for the list above:

| What | Commit |
|---|---|
| The Actor → the Performer (engine `actor` untouched) | `e91a65a38` |
| `trap_probe`, and the two findings it was built for | `42b7a5dbb` |
| Three falsified hypotheses (button hold, stick, rollback) | `fbcb01976` |
| The `PlayerVisual` gate: the hide and the door reach an actor | `7564f22bc` |
| Stage five: one authority for exit velocity, and it fires | `3f2ae723b` |
