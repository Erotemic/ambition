# Frame-aware unified movement kernel

> **Binding decision:** [ADR 0024](../../adr/0024-frame-aware-unified-movement-kernel.md).
> **State:** LANDED, including the frame-authority migration (commits
> `17685105` → `477700e9`). This document records the invariants and ownership
> of the shipped architecture, plus the honest residual debt.

## The law

Every body-relative operation during a movement tick — controller
interpretation, the active policy, jumps/dashes/blinks, knockback and launch
directions, support publication — consumes ONE environment-resolved
reference/acceleration frame for that body and tick. No subsystem
reconstructs a gravity direction, body basis, or "close enough" frame.

`MotionFrame` is one immutable value with two independent parts: an
orthonormal reference basis (`AccelerationFrame`: `side`, `down`, world-space)
and the complete world-space acceleration for the tick. Zero acceleration
retains the environment-supplied orientation; lateral/inertial acceleration
does not rotate the basis. A frame change is not a model change and resets no
private state; a model change is not a frame change and writes no environment.

## The frame resolution phase

`FrameResolveSet` (configured after `GravitySet::ZoneSnapshot`, before
`SandboxSet::CoreSimulation`) runs ONE system,
`resolve_body_motion_frames`, which publishes every integrated body's
**`ResolvedMotionFrame`** component through ONE composition rule,
`FrameEnv::resolve(body_aabb, gravity_response)`:

- the reference basis comes from the localized gravity direction the body's
  AABB **overlaps** (zone-or-ambient — the engine's zone-grab rule, never a
  center-point approximation);
- the gravity contribution is that direction × the body's authored response
  (`movement.gravity × gravity_scale`; an aerial body's 0 scale is the
  zero-acceleration-with-retained-orientation case);
- **`ForceZone`** contributions (wind, tractor fields) accumulate in world
  space, unscaled by the gravity response and without rotating the basis —
  basis and acceleration are independently resolved by construction.

Every consumer reads that artifact: the player brain and possessed/actor
brains (`control_down`), clone brains, the fast-fall/possession/interact
gestures, body-mode mechanics, affordance intent, combat knockback/DI and
hitbox sides, moveset volume rotation and start impulses, ability aim
(blink/dive/grapple/beam/…), projectile fire frames, the mount saddle
rotation, and both integration drivers (which pass it into `step_motion` as
`MotionStepContext::frame`). Input and physics therefore cannot disagree at a
zone boundary — there is only one value.

The artifact is transient environment output: not authored body state, not
part of `MotionModel`, declared snapshot-**derived** (restore recomputes it
from the live restored environment on the next resolution phase). The
presentation `GravityField` is a per-tick **mirror** of the primary body's
resolved frame with exactly one writer (`resolve_active_gravity`); free
bodies (ground items, projectiles) resolve inline at their single integration
site by the same body-overlap rule (`dir_for`).

## One entry, four authorities

`ambition_engine_core::movement::step_motion` is the ONLY continuous-movement
entry; dispatch is a single enum match over `MotionModel`
(`AxisSwept` / `SurfaceMomentum` / `AdhesiveCrawler`; absence is never a
policy). Besides the kernel, exactly three named authorities may move a body
(`movement/authority.rs`):

1. **`transit_body`** (+ `reconcile_transit`) — discrete teleports: blink and
   dive arrivals, mark-recall, possession's vacate-exit, respawns and room
   resets (`reset_body_clusters` routes through it), room transitions, LDtk
   hot-reload placement, the RL harness teleport. A transit reconciles every
   departure fact: support/wall contacts and cling cleared, ledge grab
   released, a riding momentum body arrives `Airborne`, an attached crawler
   arrives detached, and the §3.1 motion record collapses to the arrival.
   Axis maneuver TIMERS are kept — time facts, not place facts. The portal
   core writes the pose itself (it also moves cluster-less projectiles); its
   content adapter completes kernel-body reconciliation from
   `PortalBodyTransited` the same frame.
2. **`carry_body`** — parent-frame carry: the ledge-platform carry, the
   vortex well's pull, the portal-close straddle eviction (the one pushout
   exception, now named).
3. **`constrain_body_pose`** — absolute pins: the ADR 0020 mount saddle, the
   smb1 flagpole slide.

Impulses (knockback, recoil, pogo, yanks) are typed velocity operations that
consume the resolved frame (`set_jump_velocity`, `AccelerationFrame::launch`,
frame-rotated `vel +=` at combat seams).

## Ownership map

| Fact | Owner | Where |
|---|---|---|
| Current frame | ENVIRONMENT | `ResolvedMotionFrame`, published once per body tick by the frame resolution phase; consumed everywhere. |
| Controller intent | CONTROLLER SEAM | `ScreenAxes` → `LocalAxes`/`WorldVec2` against the SAME frame per `InputFrameMode`; `InputState` is the resolved artifact. |
| Physics policy | `MotionModel` (one per body, from spawn) | Guard-enforced non-optional. |
| Policy parameters | The policy | `AxisSweptParams` / `MomentumParams` / `CrawlerParams`; no parameter type contains a frame/environment fact. |
| Policy-private runtime state | INSIDE the variant | Axis: `AxisManeuverState` (coyote/drop-through/rebound, wall cling/climb + pre-wall window, jump/dash/blink buffers, dash/blink/dodge maneuvers, ledge grab, glide/fast-fall/hover phase). Surface momentum: attach/airborne + `SurfaceRef`/arc/`v_t`/lane. Crawler: `CrawlAttachment` (block normal or chain arc-length). No policy state exists in two places. |
| Published movement facts | `BodyMotionFacts` | The model-independent semantic projection (dashing, dodge i-frames, blink telegraph/aim/grace, wall cling/climb, glide, fast-fall, ledge engagement), republished after every step. Animation, combat gates (`body_vulnerable`), affordances, HUD, time-control, traces, sim-view, and RL observations read THESE — a non-axis body projects the default, so stale axis facts cannot leak. |
| Support | `MotionStepResult::support` | `SupportFact { Airborne / Supported(Contact) / Attached(Contact) }`, selected by contact **kind** (`Support`/`Head`/`Side`/`Attachment`, assigned frame-relatively at generation) — never contact-list ordering. `surface_normal` derives from it (frame-up fallback while airborne); feet-snap contacts carry the surface's TRUE face normal (≡ `-gravity` for cardinal frames, still the surface fact under oblique ones). |
| Shared body state | Body clusters | Pose, velocity, facing, size, body mode, abilities, resources (dash charges, air jumps, cooldowns), health, combat, `carried_run` (world-imparted momentum), ladder timers (body-mode mechanics'). |

## Transition semantics

`switch_motion_model(model, spec)` (no cluster access — the state lives in
the variant): same-variant refreshes parameters and preserves ALL private
state by construction; cross-variant installs a fresh destination variant
(axis: default maneuver state; momentum: `Airborne`; crawler: detached — no
nearest-surface guessing) while every shared body fact survives untouched.
Controller-independent; the worn re-wear path uses it.

## The crawler (arbitrary-angle attachment)

`CrawlAttachment::Block { normal }` clings to cardinal AABB faces with
surface-basis probe constructions (no world-axis cases; the detached fall
uses the shared axis-role sweep under `frame.acceleration()`).
`CrawlAttachment::Chain { chain, s }` clings to `SurfaceChain` polylines —
the same arbitrary-angle geometry the momentum policy rides: crawling
advances the arc-length cursor through the chain's own local frame, corner
transit IS the polyline walk, open ends detach into the covariant fall,
closed chains wrap, and a falling crawler is captured by adhesion (projection
touch on the rideable side). Landing on blocks attaches to the SUPPORT
contact's true normal, not the frame's anti-down. Evidence includes a full
circumnavigation of a chain island rotated by an arbitrary angle, seated one
half-thickness off the oblique surface through all four corner transits.

## Snapshots

`MotionModel` round-trips identity + params + ALL private state (axis
maneuver state including a mid-flight ledge grab; the crawler attachment
enum; ride state). The environmental frame and `BodyMotionFacts` are declared
derived — recomputed from the live world after restore.

## Guards (all poison-tested)

- `engine.movement-model-is-never-optional`
- `engine.crawler-flag-is-spawn-selection-only`
- `engine.pose-writes-are-authority-only` — bare `kin.pos` writes outside the
  kernel/authorities/sanctioned domain integrators are rejected.
- `engine.mechanics-consume-the-resolved-frame` — `dir_at`/`GravityField`
  reads in mechanics dirs are rejected; observation (render, roll easing,
  dev, gravity machinery) is allowlisted by path.
- Solver privacy is `pub(crate)` visibility, enforced by rustc.

## Evidence highlights

Frame: one resolution per body per tick with probe-tested schedule ordering
(a `PlayerInput` consumer observes the same tick's zone-resolved frame); a
zone-straddling body resolves by overlap for every consumer; two bodies in
different fields get their own frames; zero-response bodies keep orientation;
force zones compose without rotating the basis; clones consume their own
frame. Kernel: three-policy covariance under arbitrary rotation;
frame changes preserving private state; cross-policy swaps initializing only
destination state; same-variant refresh preserving state. Support: a wall
graze never masquerades as support; attachment/airborne facts are semantic.
Authorities: transit reconciliation (contacts/attachment cleared, time facts
kept, motion record collapsed); carry/constrain leaving contact facts alone.
Crawler: cardinal island circumnavigation on all four faces; oblique-frame
landing attaching to the surface's true normal; arbitrary-angle chain-island
circumnavigation. Plus the app-level rl_sim reachability/parity/desync-canary
matrix.

## Residual debt (honest)

1. **Portal transit orientation** reads the presentation `GravityField`
   mirror rather than per-body frames (the portal core also moves
   cluster-less projectiles). Behavior matches pre-campaign; portal rooms do
   not currently nest gravity zones. Closing it wants a per-body frame port
   on the transit policy.
2. **`GravityField`/`GravityZones`/`ForceZones`** remain unregistered
   snapshot resources (reviewed inventory rows; restore recomputes frames
   from the live field by construction, but rewinding a mid-rewind gravity
   switch is not covered).
3. **`ControlFrame`** (the device latch) still carries raw `f32` axes; typed
   `ScreenAxes` begin at the resolution seams that consume it.
4. **Sanic ball dash** writes `SurfaceMotion::Riding { v_t }` directly from
   demo content; a typed tangential-impulse op on `SurfaceMomentumMotion`
   would close it.
5. **Block→chain crawl transfer**: a block-attached crawler does not migrate
   onto an overlapping chain (and vice versa) without detaching first; the
   two surface domains are authored separately today.
