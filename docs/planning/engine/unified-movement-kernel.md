# Frame-aware unified movement kernel

> **Binding decision:** [ADR 0024](../../adr/0024-frame-aware-unified-movement-kernel.md).
> **State:** LANDED (commit `17685105`). This document records the invariants
> and ownership of the shipped architecture, plus the honest residual debt.

## The law

Every movement tick is interpreted in the body's **current acceleration/
reference frame**, resolved by the environment exactly once per body tick.

`MotionFrame` is one immutable value with two independent parts: an
orthonormal reference basis (`AccelerationFrame`: `side`, `down`, world-space)
and the complete world-space acceleration vector for the tick. Ordinary
gravity aligns them; neither is derived from the other at the trusted
boundary. Zero acceleration retains the environment-supplied orientation;
lateral/inertial acceleration does not rotate the basis.

A movement policy never caches, authors, snapshots, or reconstructs the
current frame. A frame change is not a model change and resets no private
state; a model change is not a frame change and writes no environment.

## Ownership map

| Fact | Owner | Where |
|---|---|---|
| Current frame (basis + acceleration) | ENVIRONMENT | `GravityCtx::motion_frame_at(pos, response)` — localized zone/ambient direction × the body's authored response (`movement.gravity × gravity_scale`; a zero-scale flyer is the zero-acceleration-with-orientation case). Resolved once per body tick by the integration drivers (`integrate_sim_bodies`, `integrate_boss_bodies`) and passed unchanged to input projection and the policy. |
| Controller intent | CONTROLLER SEAM | Raw device axes are `ScreenAxes`; the seam resolves them against the SAME frame into `LocalAxes` (body-local) or `WorldVec2` (world-space, e.g. blink aim) per the user's `InputFrameMode` policy. `InputState` is the resolved intent artifact; every directional field carries its frame in its type. Screen/body-relative preference is controller policy, never a movement parameter. |
| Physics policy | `MotionModel` (one per body, from spawn) | `AxisSwept(AxisSweptMotion)` / `SurfaceMomentum(SurfaceMomentumMotion)` / `AdhesiveCrawler(AdhesiveCrawlerMotion)`. Absence is never a policy; integration queries take the component as REQUIRED and a workspace-policy guard rejects `Option<&MotionModel>` / `Without<MotionModel>`. |
| Policy parameters | The policy | `AxisSweptParams` (grouped: `AxisLocomotion` + `TraversalAbilityTuning` + `FlightTuning`), `MomentumParams`, `CrawlerParams`. No parameter type contains gravity direction, acceleration, reference orientation, or input-frame preference. `MovementTuning` remains the flat AUTHORING aggregate; its `gravity` field is the authored response magnitude fed to the resolver, not a policy input. |
| Policy-private runtime state | The policy | Surface momentum: attach/airborne, `SurfaceRef` + arc `s` + signed `v_t`, depth lane. Crawler: the attachment normal (`CrawlerState`), mutated externally only via the typed `detach()` op. Axis: coyote/buffers/wall/ledge/dash/blink maneuver state — physically still on the per-body cluster components (see debt), but OWNED by the policy: `switch_motion_model` initializes it on cross-model entry and no other policy reads it. |
| Shared body state | Body clusters | Position, velocity, facing, size, body mode, abilities, resources, health, combat — everything whose meaning survives a policy change. |

## One entry

`ambition_engine_core::movement::step_motion` is the ONLY movement entry.
Dispatch is a single enum match inside the kernel. Home bodies, actors,
bosses, possessed bodies, clones, RL bodies, demo bodies, and tests all reach
it; the ECS drivers only gather state, resolve the frame and intent, call it
once, and publish the result (`MotionStepResult`: `FrameEvents` + the support
`surface_normal`, mirrored into `ActorSurfaceState` for every body by one
rule). Phase helpers and individual solvers (`step_surface_body`,
`step_crawler`, the axis phase functions) are kernel-private; the historical
whole-policy `update_player_*`/`update_body_*` entries are deleted (engine
tests drive `step_motion` through the crate-private `test_support` module).

Side paths, resolved:

- **`surface_walker`** → the `AdhesiveCrawler` policy (kernel-owned crawl,
  corner transit, moving-surface carry, detached fall through the shared axis
  collision doctrine under `frame.acceleration()`). The authored
  `surface_walker` boolean is spawn-time policy SELECTION only
  (`ActorTuning::motion_model`), guard-enforced.
- **Moving platforms** — contact kinematics consumed inside the kernel
  (support velocity carry; crawler cling carry), plus the axis-model-private
  ledge-platform carry in the home integrator.
- **Water/climbing/flight/blink/ledge/fast-fall** — policy modes and typed
  ability verbs consumed inside the axis policy.
- **Knockback/recoil/impulses** — additive world-space velocity writes outside
  the tick, consumed by the next kernel step; launch directions use the
  environment-resolved direction, never a tuning field.
- **Crawler anti-clump reversal** — controller-side steering intent in the
  driver (the kernel only moves).

## Transition semantics

`switch_motion_model(model, spec, clusters)` is THE runtime swap:

- **Same-variant** — parameters refresh; ALL private runtime state survives
  (ride surface/arc/speed/lane, crawler attachment, axis timers).
- **Cross-variant** — every shared body fact survives untouched; ONLY the
  destination's private state is initialized: axis enters with empty
  support/contact caches and no in-flight maneuver (resource counts and
  recharge cooldowns are body facts and survive); surface momentum enters
  `Airborne` on the unchanged pose (attachment only via its own same-tick
  contact rules); the crawler enters detached (no nearest-surface snapping).
- The operation reads no controller, no environment; swapping is
  controller-independent by construction. The worn-character re-wear path
  (`apply_worn_character_gameplay`) goes through this seam.

## Snapshots

`MotionModel` is registered in the N3.1 ledger (`motion_codec.rs`): policy
identity + authored params + private state round-trip; the environmental
frame is deliberately NOT encoded — after restore the next tick resolves it
from the live restored environment.

## Guards

- `engine.movement-model-is-never-optional` — no production
  `Option<&MotionModel>` / `Without<MotionModel>` anywhere in `crates/` or
  `game/`.
- `engine.crawler-flag-is-spawn-selection-only` — `tuning.surface_walker` is
  readable only by the spawn selector.
- Both poison-tested (`movement_kernel_guards_react`); solver privacy is
  `pub(crate)` visibility, enforced by rustc.

## Evidence

Pure kernel tests (`movement/kernel/tests.rs`): three-policy arbitrary-angle
covariance; zero-acceleration retained orientation; lateral acceleration not
rotating the basis; frame rotation/time-variation preserving each policy's
private state; cross-policy round trips preserving shared state and
initializing only destination state; crawler convex-corner wrap with the
published support fact. Assembled tests: worn re-wear same-model ride
preservation and cross-model swap semantics (`live_refresh.rs`), crawler
cling-break typed detach (damage tests), slug moving-platform carry through
the kernel (conversion tests), snapshot policy round-trip
(`ambition_runtime` snapshot tests), Sanic loop/momentum acceptance through
`step_motion`, and the Mary-O/host/app suites.

## Residual debt (honest)

1. **Axis private state placement.** Coyote/buffer/wall/ledge/dash/blink
   maneuver state still lives physically on per-body cluster components
   (individually snapshot-registered). Ownership and cross-model
   initialization are kernel-enforced; physically relocating the fields into
   `AxisSweptMotion` would collapse several ledger rows and is deferred until
   the next snapshot-ledger pass.
2. **`GravityField`/`GravityZones`** remain unregistered snapshot resources
   (pre-existing rollback debt; the frame law makes restore use the live
   field by construction, but rewinding a mid-rewind gravity switch is not
   yet covered).
3. **Crawler covariance** is proven for detached fall at arbitrary angles;
   attached crawling is cardinal-frame like the AABB world it climbs.
4. **`ControlFrame`** (the device latch) still carries raw `f32` axes; typed
   `ScreenAxes` begin at the resolution seams that consume it.
5. **Sanic ball dash** writes `SurfaceMotion::Riding { v_t }` directly from
   demo content — a content-side ability authoring policy state; a typed
   tangential-impulse op on `SurfaceMomentumMotion` would close it.
