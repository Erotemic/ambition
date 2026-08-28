---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/systems/headless-simulation.md
  - docs/concepts/testing-and-validation.md
---

# Headless room verification

Use the real provider/runtime construction path. A hand-built `World` that skips
loading, lifecycle, control, or lowering can prove a unit invariant, but it does
not prove the assembled game.

## Localize the existing seam

```bash
python scripts/agent_query.py "headless room load provider"
python scripts/agent_query.py tests "<room id or behavior>"
python scripts/agent_query.py crate ambition_sim_harness
```

Prefer extending an existing harness/helper over adding a second miniature app.

## Verification ladder

1. **Pure/domain test** — prove the narrow transformation or policy.
2. **Owning-crate assembled test** — construct the smallest real plugin/domain
   composition.
3. **Provider/runtime test** — prepare the provider, load the room through the
   transaction, step simulation, and observe stable read models/traces.
4. **Geometry render** — inspect authored collision/entities without a GPU.
5. **Visible smoke** — reserve for presentation feel that cannot be inferred
   from authoritative state.

## Commands

```bash
python scripts/agent_query.py tests "<invariant>"
./run_tests.sh -k <test_substring>
./run_tests.sh -p <owning_package> -k <test_substring>
cargo run -p ambition_platformer2d_actor_monolith --example render_room_geometry -- <ROOM_ID>
```

`game/ambition_app` integration tests are aggregated through the current
`app_it` test surface; do not invoke deleted historical test binaries. Let
`./run_tests.sh` and `agent_query.py tests` choose the current package/test shape.

## What to assert

Assert durable outcomes:

- provider/session/room construction commits atomically;
- expected stable IDs resolve and scoped entities exist;
- the actor receives semantic control and uses the shared body/action path;
- movement, interaction, hit, transition, or progression facts occur;
- cleanup/reset removes the old scope and derived views converge;
- gravity rotations or equivalent symmetries preserve the mechanic;
- the test is non-vacuous (the target event/state was actually reached).

Avoid pinning exact frame counts, velocities, coordinates, or visual assets unless
they are the contract under test.

## Seeing a picture, and which renderer to reach for

Three paths exist and they are not interchangeable, but the gap is **0.43 s vs
167 s**, not "seconds vs never": the geometry render below was measured at
0.43 s on 2026-08-02, and `capture_scene` at **167 s on 2026-08-09** —
⚠ **re-measured, because this line used to say it never finished at all.** See
§2 for the correction and why the stale version was expensive.

⇒ pick by WHAT YOU NEED TO SEE, not by cost: geometry draws boxes without art,
`capture_scene` draws the real thing. **A question about art cannot be answered
by the fast one at any price.**

### 1. Geometry — `render_room_geometry` (sub-second, no GPU)

```bash
cargo run -p ambition_platformer2d_actor_monolith --example render_room_geometry -- <ROOM_ID> [OUT.png]
cargo run -p ambition_platformer2d_actor_monolith --example render_room_geometry           # lists every room
```

A pure pixel buffer — no wgpu, no windowing, no display. It draws collision and
volume BOXES in world space, not sprite art, which is exactly the right
instrument for "where is this thing" questions: room bounds, spawns, hurtbox vs
body envelope, mid-air doors.

**Reach for this first.** It is the answer to most questions that feel like they
need a screenshot.

### 2. The real render stack — `capture_scene`

```bash
cargo run -p ambition_app_tools --bin capture_scene -- <ROOM_ID> <X,Y|player> [OUT.png] [WxH] \
    [--warmup N] [--combat-overlay] [--press KEYS] [--character ID] [--route ID]
```

This runs the actual presentation plugins, so it is the only path that shows
sprite ART. Two flags matter for combat work:

- `--combat-overlay` puts the `DebugViewMode::Combat` preset in the shot. The
  volumes are off by default, so without it a swing is photographable and its
  hit polygon is not.
- ⛔⛔ **A SMASH MOVE IS PHOTOGRAPHED ON THE SMASH STAGE.** Jon, 2026-08-28:
  *"when we are doing smash moves we probably should be using the smash stage
  and not any ambition stages, to make sure that we're actually getting smash
  rules and not ambition which might be different."* `--route smash_gameplay
  --character <id>` seats a match of two of that character and comes up under
  the match ruleset — stocks, damage percent, the lot. Without `--character` the
  route activates a stage with nobody on it (D130 recorded that once as a
  mystery). The exploration form — `capture_scene <room> player --character
  <id>` — puts the same body under EXPLORATION rules, which is the wrong game to
  judge a fighter's move in.
- `--press` drives input, and takes `hold:KEY` / `release:KEY` as well as taps.
  Every tilt and aerial is *a direction held while attack is pressed*, so
  `--press up,x` taps Up, releases it, and then attacks — resolving forward.
  `--press hold:up,x,release:up` is an up-tilt; `z,wait:10,hold:down,x,release:down`
  is a down-air. Presses restart the capture clock, so `--warmup N` after them
  reads as N sim ticks into the move (the phase colour tells you whether you
  landed in the active window: red is active).

✔✔ **IT WORKS, AND THIS PARAGRAPH USED TO SAY IT DID NOT. Re-measured
2026-08-09: 167 s wall, image written.**

```
$ /usr/bin/time cargo run -p ambition_app_tools --bin capture_scene -- \
      hall_of_characters player out.png 640x360 --warmup 60
capture_scene: wrote out.png (640x360 px)
WALL=167.30 s          # on a box already at load average 15
```

⛔ **the stale ⛔ below cost real verification.** A worker fixing the goblin
encounter on 2026-08-09 skipped its visual check and cited this line — for a bug
whose entire symptom is visual. **A recipe that wrongly forbids a tool is worse
than no recipe**, because it is believed.

<details><summary>What the old measurement said, kept so the change is legible</summary>

> Measured: forty minutes, single-threaded, spinning CPU, no log line, no
> graphics library mapped and no asset file opened. That profile pointed at
> startup work in a 688 MB debug binary (the baked sheet RONs are
> `include_str!`'d), NOT at the GPU — software Vulkan is installed
> (`/usr/share/vulkan/icd.d/lvp_icd.json`, lavapipe), and there is no `DISPLAY`,
> no `/dev/dri` and no Xvfb.

</details>

⚠ **what changed is not established.** `2f9fd6085` moved six binaries out of
`ambition_app` (21.7 s → 15.1 s) and is the obvious candidate, but nobody
bisected it. ⇒ **if it hangs for you, re-measure and say so with a number rather
than restoring the old paragraph** — that is what let this one sit wrong.
⚠ the 167 s is a WARM build plus the run. A cold build is on top.

### 3. Art against its own volume — composite it yourself

When the question is specifically "does the drawn effect match the volume that
hurts", neither of the above answers it: one draws boxes without art, the other
draws art without finishing. Reproduce the runtime's mapping in a few lines of
Python instead — read the sprite sheet and the manifest polygon, project the
volume the way `CombatVolume::swing_shape` does, stretch the sprite into that
quad, and draw both into one image.

⚠ **A harness that models the pipeline minus one step is a confident wrong
answer, not a weaker one.** A containment measurement built this way reported
0.00% of the slash outside its polygon while a screenshot of the same frame
plainly showed ink past the outline: it reproduced `swing_shape` and not the
`SLASH_ART_MARGIN` applied one line later. Check any such harness against a
picture before believing a number from it.

### Finding this page

`python scripts/agent_query.py "headless screenshot"` reaches it. Going straight
to the binary that sounds right instead cost most of a session.
