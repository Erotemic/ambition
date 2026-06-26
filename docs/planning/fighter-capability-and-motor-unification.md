# Fighter capability + motor unification — toward a faithful in-engine N-fighter match

Status: **proposal** (not started). Author model: Opus 4.8 (1M).
Prompted by: the "two hybrid fly/ground PCAs in the symmetry chamber" request.

## Why this exists

The advanced fighter brain (duelist footsies, aerial dive/perch, blink, block,
hybrid fly/ground) is built and proven by the headless arena harness
(`brain/smash/arena.rs`). But the harness is currently a **brain-policy proxy**,
not a faithful preview of the in-engine fight: several verbs the brain emits do
not resolve on an enemy body, and the harness models movement with its own
kinematics. Building "two PCAs that fight in the C4 symmetry room" *the same way
in-engine as in the test* hits three architectural seams. This doc proposes the
smallest set of changes that makes an N-fighter match faithful and elegant —
and, as a bonus, removes a movement bifurcation and unblocks rotated-gravity
flight.

The goal isn't more verbs; it's **one pipeline** so that "put two of them in an
arena" is literally two bodies with the same capabilities + brain, behaving
identically whether driven headless or in the game.

## The three frictions (evidence from the build)

### F1 — Movement abilities have no capability-resolution layer (the big one)

Attacks already have a clean capability-vs-policy split: the brain emits abstract
intent (`melee_pressed`, `fire`, `special_pressed`) and the body's `ActionSet`
resolves it to a concrete effect (`ActorActionMessage`). **Movement abilities
have no equivalent.** The brain emits `blink_pressed`, `fly_toggle_pressed`,
`shield_held`, `dash_pressed` on `ActorControlFrame`, but the enemy integrator
(`features/enemies/integration.rs`) consumes only `locomotion` / `velocity_target`
/ `jump_pressed` / `drop_through`. Those ability edges resolve **only for the
player body** (which carries the blink/fly/shield ability clusters).

Consequence: the in-game PCA's blink, block, and hybrid fly-toggle are
*emitted-but-inert*. The commit that wired the PCA to fly had to say so. The kit
is real in the harness and dead in the engine — the harness and the game have
diverged.

### F2 — Grounded vs aerial motor is bifurcated in the brain

`tick_smash` writes grounded `locomotion` + `jump_pressed`, then for an aerial
body *discards* them post-emit and writes `velocity_target` from `aerial_steer`.
Two parallel movement vocabularies for one concept ("move toward a desired
spot"). The hybrid fly/ground toggle has to straddle both. It works, but the
seam is a smell: a flyer's "approach" and a walker's "approach" are the same
intent expressed twice.

### F3 — Flight steering isn't frame-agnostic (blocks C4)

The grounded path is gravity-relative (`BrainSnapshot::acceleration_frame()` /
`target_delta_local()` — it walks along walls under sideways gravity). But
`aerial_steer` and the blink evade assume `-y` is "up" (perch above = negative
y, dodge up = `-1.0` y). Under the symmetry room's rotated (C4) gravity, "up"
isn't `-y`, so flight + the up-and-away blink would misbehave. The two-PCA
chamber test runs default down-gravity today for exactly this reason. This is a
relativity-principle violation introduced by the aerial work.

(Prerequisite, logged separately as a smell: enemy `BrainSnapshot.sim_time` is
hardcoded `0.0`, so the reaction-latency `obs_history` is inert in-engine. A
faithful in-game match needs the sim clock threaded.)

## Proposed shape

Three changes, independent, smallest-blast-radius first.

### P3 (do first) — frame-local flight steering

Express `aerial_steer`'s perch/dive offsets and the blink "up-and-away" vector in
the **acceleration frame** (`control_down`), the same way the grounded stages
already do. "Above the target" becomes `-control_down`; the perch offset is built
in local axes and `to_world`'d. Pure brain-crate change, no integration touch.

Unlocks: flight works under any gravity orientation; the chamber test can run
under all four C4 orientations (a strong relativity-principle guard, very
on-theme for the Noether Chamber). Cheap, isolated, high-signal — the natural
first slice.

### P1 (keystone) — a movement-ability capability layer

Mirror `ActionSet` for movement. A per-body `MovementCapabilities` (or extend the
existing ability clusters) that resolves the brain's movement-ability edges into
the body's actual effect, so the integrator reads *resolved* intent rather than
special-casing the player:

- `blink_pressed` + `blink_quick_dir` → a teleport, **if** the body has the blink
  capability (cost/cooldown owned by the capability, like the player's).
- `fly_toggle_pressed` → flip the body's `gravity_scale` (aerial⇄grounded), if it
  has the fly capability.
- `shield_held` → raise a guard / damage-reduction state, if it can shield.
- (`dash_pressed`, `drop_through` already partially handled — fold in.)

The player's existing blink/fly/shield are factored so an enemy body can opt into
the same capability. Then "the PCA blinks/blocks/toggles flight in-engine" is a
capability the archetype carries, not new integration code — exactly the
pay-for-use, possession-ready split attacks already enjoy.

Unlocks: the in-engine PCA gets its full kit; **the harness and the game run the
same brain → same resolved verbs**; two PCAs are two bodies with identical
capabilities; possession of a PCA "just works" (the player inherits its kit).

### P2 (cleanup) — unify the motor projection

Have the brain emit a single frame-agnostic **desired body-local velocity** (plus
the ability edges), and one projection map it to the body's current mode:
grounded → `locomotion` throttle + a `jump_pressed` when the desired vertical
exceeds a hop, free-mover → `velocity_target`. The grounded footsies and the
aerial dive/perch become one "desired relative position → motor" with two
projections, not two vocabularies. The hybrid toggle then only changes *which
projection* runs, and F2 disappears.

Unlocks: removes the post-emit override hack; hybrid flight is a one-line mode
switch; new locomotion modes (wall-cling, water) slot into the projection, not
the brain.

## What it unlocks together

- **Faithful N-fighter match**: the arena harness stops being a policy proxy and
  becomes a true headless preview — same brain, same capabilities, same resolved
  verbs as the game. "Two PCAs in the symmetry room" is one code path.
- **Full kit in-engine**: blink / block / hybrid fly all resolve on the PCA body.
- **C4 / any-gravity flight**: the symmetry room's whole point works.
- **Possession parity**: possessing a PCA inherits its movement kit for free.

## Migration sketch (incremental, each shippable)

1. **P3** — frame-local `aerial_steer` + blink; add a chamber test under the 4 C4
   orientations. Brain-crate only. (S–M)
2. **sim_time** — thread the scaled clock into `build_enemy_brain_snapshot`
   (smell already logged); un-inerts reaction latency in-engine. (M)
3. **P1** — `MovementCapabilities` resolution for blink/fly/shield on any body;
   give the PCA archetype the capabilities. Biggest change; do behind tests that
   assert an enemy body with the capability actually blinks/toggles. (L)
4. **P2** — unify the motor projection; delete the post-emit aerial override. (M)

## Non-goals

- The RL seam (unchanged — a learned policy still emits the same
  `ActorControlFrame`; P1/P2 make its verbs resolve identically too).
- New abilities. This is plumbing the existing kit through one pipeline, not
  adding verbs.
- The Conway-glider visual (S4 content art).

## Pointers

- Brain: `crates/ambition_characters/src/brain/smash/` (`mod.rs` tick + steers,
  `arena.rs` harness), `actor/control.rs` (the frame), `brain/action_set/`
  (the attack capability layer to mirror).
- Integration: `crates/ambition_gameplay_core/src/features/enemies/integration.rs`
  (what an enemy body currently consumes), `features/ecs/actors/update.rs`
  (`build_enemy_brain_snapshot`).
- Design note: `docs/design/pca-fighter-brain.md`. Smells:
  `dev/journals/code_smells.md` (2026-06-26 sim_time / wall_contact).
