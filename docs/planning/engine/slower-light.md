# Slower Light — the reduced-speed-of-light mechanic (stretch, seams now)

> **Status: Jon's directive (2026-07-05), design note by fable.** Jon:
> *"I would also like to add a 'slower speed of light' mechanic where you can
> lower the speed of light. I'm not sure how feasible this is in 2D but I
> think we could use shaders to modify light — the trick is how to warp
> space. This is again a stretch goal but I want the core system to make it
> easy when the time comes. (and it will)"*
>
> Adjudicated as **AJ14** in the archived 07-05 plan
> ([`../../archive/reviews/fable-demo-plan-2026-07-05.md`](../../archive/reviews/fable-demo-plan-2026-07-05.md));
> the live queue is [`../tracks.md`](../tracks.md).
> Nothing here builds the mechanic today; it fixes the SEAMS so building it
> later is a content-plus-shader arc, not an engine rewrite. Direct kin of
> [`frame-awareness.md`](frame-awareness.md) ("future relativity-inspired
> mechanics") and the north star ("every biome a math world model").

## Feasibility (short answer: yes, and 2D is mostly EASIER)

Prior art exists: MIT Game Lab's *A Slower Speed of Light* / OpenRelativity
does this in 3D with vertex + fragment shaders. Every piece maps to 2D, and
several get simpler:

| Effect | What it is | 2D difficulty |
|---|---|---|
| Speed cap `c` | nothing exceeds c; bodies asymptote toward it | trivial (a velocity clamp) |
| Time dilation | a fast body's proper time runs slow by `γ = 1/√(1−v²/c²)` | **already built** — ADR 0010/0011 proper time |
| Length contraction | moving bodies squash along motion | a sprite/vertex scale along the velocity axis |
| Doppler shift | blue ahead / red behind when the OBSERVER moves | a full-screen color LUT post pass |
| Aberration ("warp") | the view compresses ahead, dilates behind | a full-screen UV remap post pass |
| Light delay | you SEE things where they WERE | render/perceive from a short position history |

**The key adjudication — "how to warp space":** you don't. The SIM keeps one
honest, unwarped world (one Galilean sim + a speed cap + per-body time
dilation). The "warp" is what the **observer sees** — an observer-frame view
transform applied at the camera boundary, exactly the AJ13 discipline
(*a camera is not the world; it is an observer*). This is also how the MIT
game does it. Sim-side space warping (light-cone-limited collision,
per-observer geometry) is explicitly out of scope — it would fork the sim
and buys almost no gameplay the observational version doesn't.

That makes the mechanic's gameplay come from three REAL sim effects — the
cap (your dash/run/projectiles crowd toward c), dilation (moving fast slows
your own clocks: move cooldowns, timers — the moveset already runs on proper
time), and delayed information (you and the AI react to where things WERE) —
while the drama (warp, Doppler, contraction) is presentation.

## Why Ambition is unusually ready for this

- **Per-body proper time is load-bearing already**: `ProperTimeScale` +
  `WorldTime` + the moveset's proper-time clock (ADR 0010/0011). Relativistic
  dilation is a small system writing `γ(v, c)` through the EXISTING seam
  (`ClockScaleRequest` discipline — never mutate time_scale directly).
- **Perception is a typed policy** (`Perception::Omniscient/Sighted`, R1.2b).
  Light-limited AI is a third variant (`LightLimited { c }`) whose WorldView
  is built from retarded state — brains chase where you WERE. Same seam,
  new policy.
- **Camera-as-observer is becoming structural** (AJ13 + the E4
  `ambition_sim_view` carve): the renderer already will consume a read-model
  snapshot, which is exactly where an observer-frame transform plugs in.
- **Zones-as-fields precedent**: `GravityZone` shows the shape — `c` is a
  world/zone parameter (`LightZone { aabb, c }`), infinite by default, so
  every existing room is the classical limit and pays nothing.
- **Frame discipline** (frame-awareness.md) is the mental model: γ is a
  relationship between a body's frame and the zone's rest frame; the Doppler
  and aberration passes are functions of the OBSERVER's velocity in that
  frame.

## The staged plan (each tier independently shippable)

- **Tier 0 — the seams (NOW, cost ≈ zero; see "now-obligations")**: no code,
  just requirements on in-flight work so nothing hardcodes against this.
- **Tier L1 — sim kinematics [opus, ~1 session when scheduled]**:
  `LightZone { aabb, c }` (+ a world default, `f32::INFINITY` = classical);
  a velocity clamp toward c applied at the shared integrate seams (relax
  rule like the fall cap — never brake an already-over-c fling, asymptote
  new acceleration); a `relativistic_time_dilation` system computing
  `γ = 1/√(1−v²/c²)` and writing the body's proper-time scale through the
  ClockScaleRequest/ProperTimeScale seam. **Headless-testable, C4-style: the
  twin test** — two bodies, one loops at 0.9c, assert its accumulated proper
  time = the stationary twin's × 1/γ within tolerance. Frame-agnostic by
  construction (all dot products).
- **Tier L2 — light-limited information [opus after L1]**: a short
  `KinematicHistory` ring buffer (opt-in component, only inserted inside a
  finite-c zone) + `Perception::LightLimited { c }` building the WorldView
  from each peer's retarded position (`t_ret` solved by 2–3 fixed-point
  iterations of `|x_peer(t_ret) − x_self| = c·(t − t_ret)`). AI now dodges
  your PAST. HUD/targeting for a player body reads the same retarded view.
- **Tier L3 — the observer shaders [opus + BLIND feel; the "warp"]**: a
  full-screen post pass fed by the sim_view observer velocity:
  (a) **aberration/contraction UV remap** — remap screen x (and optionally y)
  by the relativistic aberration map for v_observer/c (this warps EVERYTHING
  — tiles, sprites, background — in one shader, which is why the post-pass
  beats per-sprite warps); (b) **Doppler LUT** — hue shift by the per-pixel
  approach velocity sign/magnitude (cheap approximation: screen-space
  direction relative to observer motion); (c) optional per-body length
  contraction on sprites moving relative to the observer (vertex squash
  along their velocity — reads the read-model velocities). Retarded-position
  RENDERING (drawing bodies from the L2 history) is the last, most
  disorienting toggle — ship each sub-effect behind its own knob for Jon's
  feel pass.
- **Tier L4 — content**: the relativity biome/zone ("every biome a math
  world model"; the ability-as-theorem here is literally γ), puzzles built
  on dilation (outrun a timer by moving fast), delayed-information combat,
  and a boss that exploits your light cone.

## The now-obligations (Tier 0 — folded into the live plan)

1. **E4 (`ambition_sim_view`) requirement**: the read-model carries, per
   rendered body, **position AND velocity** (world-frame, named per AJ13) and
   exposes the **observer's velocity** (the controlled body's, or the camera
   rig's) as part of the camera snapshot. Rationale recorded in the E4 spec:
   L3's post passes are pure functions of these; if the read-model ships
   position-only, the warp arc starts with a schema break.
2. **History-sourced views stay possible**: `WorldView` construction and the
   sim_view snapshot builder must not be written in a way that assumes
   "state == live ECS state this frame" is the ONLY source (no new global
   singletons that alias live state; keep the build funnels — `build_world_view`,
   the snapshot builder — as functions OF inputs). No code change today;
   this is a review-flag note (it is ALREADY true; keep it true).
3. **Speed caps stay seam-shaped**: new movement code keeps caps as
   parameters/rules at the shared integrate seams (the existing pattern),
   never scattered magic constants — L1's c-clamp lands at those seams.
4. **Post-process hook**: the render stack keeps (or gains, when E4 lands) a
   single full-screen post seam where ordered passes can be registered —
   L3 registers there. (Bevy supports this natively; the obligation is just
   not to bury the camera output where a post pass can't see it.)
5. **Naming**: the mechanic's vocabulary is `LightZone` / `c` /
   `LightLimited` / retarded state — recorded here so slices don't invent
   competing terms.

## Non-goals (explicit)

- No sim-side space warping, no per-observer collision geometry, no
  light-cone causality in the SIM. One honest world; observers see it bent.
- No general-relativity (curved-space) mechanics — gravity zones already
  cover the gameplay want there; if a curved-space biome ever matters it is
  its own manifesto.
- Multiplayer + finite c interactions (whose observer frame wins?) —
  deferred until multiplayer scope (roadmap Q5) is real; single-observer
  (the controlled body) is the design center.

## Pointers

- Proper time: ADR 0010/0011, `WorldTime`, `ProperTimeScale`, the moveset's
  `entity_dt` (combat/moveset/mod.rs — the proper-time clock).
- Perception policy: `features/ecs/perception.rs` (`Perception` enum).
- Zone precedent: `GravityZone` (platformer_primitives gravity.rs).
- Observer boundary: E4 / `ambition_sim_view` in the demo plan (track E).
- Prior art: MIT OpenRelativity / *A Slower Speed of Light* (Unity, open
  source) — the effect decomposition above mirrors theirs.
