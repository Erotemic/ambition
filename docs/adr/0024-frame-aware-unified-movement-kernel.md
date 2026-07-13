# ADR 0024: One frame-aware movement kernel with explicit swappable policies

## Status

Accepted; implemented, INCLUDING the frame-authority migration
(commits `17685105` → `477700e9`).

Mechanically enforced today:

- ONE frame resolution per integrated body per tick: the frame resolution
  phase publishes `ResolvedMotionFrame` (basis + accumulated acceleration
  contributions, body-overlap zone selection) and every body-relative
  consumer — controller interpretation, brains (incl. possessed bodies and
  clones), combat, abilities, mounts, body-mode, and both integration
  drivers — reads that artifact. Guarded:
  `engine.mechanics-consume-the-resolved-frame` (poison-tested).
- ONE continuous-movement entry (`step_motion`, `pub(crate)` solvers) plus
  three named non-kernel authorities (`transit_body` with documented
  reconciliation semantics, `carry_body`, `constrain_body_pose`); bare pose
  writes are guarded: `engine.pose-writes-are-authority-only` (poison-tested).
- Policy-private state lives INSIDE the model variant (`AxisManeuverState`,
  ride state, `CrawlAttachment`); the published `BodyMotionFacts` projection
  is the only outside read surface, so a non-axis body cannot leak stale
  maneuver facts. `MotionModel` snapshot round-trips all of it; the frame and
  the facts are declared derived.
- Support is a semantic fact (`SupportFact`, contact kinds assigned
  frame-relatively at generation), never contact-list ordering.
- The crawler attaches at arbitrary angles through `SurfaceChain` geometry
  (block faces stay probe-based over the AABB world, with surface-basis
  constructions and no world-axis cases).
- The optional-model / crawler-flag guards remain live and poison-tested.

Residual debt is recorded in
[`docs/planning/engine/unified-movement-kernel.md`](../planning/engine/unified-movement-kernel.md).

## Context

Ambition has more than one legitimate body-motion law:

- axis-swept action-platformer movement;
- Sonic-style surface momentum over authored routes;
- the existing adhesive `surface_walker`, which currently writes body position,
  velocity, support, corner transitions, and detached falling through a hidden
  actor-only path.

These laws are not parameter presets for one solver. They own different
persistent state and have different contact semantics. They nevertheless act on
the same bodies, may be selected independently of controller ownership, and must
obey the same environment and frame law.

Historically, movement identity was partly implicit: absence meant axis-swept,
outer ECS systems selected solvers, gravity direction lived in tuning, controller
vectors crossed boundaries as untyped `Vec2`, and several abilities or actor
kinds reconstructed their own acceleration frame. That architecture cannot
support arbitrary-angle, zero-force, localized, moving, or non-inertial
reference frames reliably. It also makes model swapping and deterministic
snapshot restore ambiguous.

The movement kernel is a small trusted core. Frame correctness and state
ownership are more important than preserving old APIs or minimizing the diff.
Ambition is pre-release; compatibility aliases and parallel paths are not a goal.

## Decision

### 1. Every integrated movable body owns exactly one explicit policy

The engine-core movement component is a non-optional enum. The accepted policy
set is:

- `AxisSwept`;
- `SurfaceMomentum`;
- `AdhesiveCrawler`, the destination of the current `surface_walker` integrator.

The final names may improve during the cutover, but absence is never a policy and
no outer query may interpret a missing component as axis-swept. Every home body,
actor, enemy, boss, possessed body, clone, RL body, and test fixture that enters
integration carries one variant from spawn.

`AdhesiveCrawler` is a policy, not a modifier. The current surface-walker path
moves the body, owns attachment/corner state, decides support, applies platform
carry, and performs detached falling. Treating it as a boolean constraint would
hide a third integrator behind the axis solver.

### 2. There is one public movement entry

`ambition_engine_core::movement::step_motion` is the one authoritative whole-tick
entry. Physics dispatch is an explicit match inside the trusted kernel.

The ECS layer may:

1. gather the authoritative shared body state;
2. resolve the live environment frame once;
3. resolve controller intent against that same frame;
4. gather typed external effects and medium/contact context;
5. call `step_motion` once;
6. publish the common result vocabulary.

It may not select a solver, call a policy-specific whole-tick function, or update
pose through an actor/player/demo-specific integration branch.

Policy-private functions and phase helpers remain private to engine-core. Pure
geometry helpers may be public only when they do not advance a body or constitute
an alternate movement entry.

### 3. Reference orientation and acceleration are distinct typed facts

The per-tick frame is one immutable value with two independent parts:

- a normalized reference basis expressed in world space (`side`, `down`);
- the complete world-space linear acceleration vector for the tick.

The current `MotionFrame { basis: AccelerationFrame, acceleration }` is an
acceptable migration representation. The long-term vocabulary should make the
semantics explicit (`ReferenceBasis2` and `WorldAcceleration` or equivalent);
`AccelerationFrame` must not be interpreted as permission to derive orientation
from every force vector.

The environment may therefore represent:

- nonzero acceleration aligned with the body's down axis;
- zero acceleration with a retained environment-defined orientation;
- a stable basis plus lateral or inertial acceleration;
- arbitrary-angle, localized, and time-varying fields;
- future translating or rotating non-inertial environments.

Zero acceleration never means “normal gravity.” Additional acceleration never
silently rotates the basis. A vector with force semantics may not substitute for
an orientation merely because ordinary gravity aligns them.

### 4. The environment resolves the frame once per body tick

One environment resolver composes every relevant contribution before movement:
room gravity, local fields, authored orientation, moving/rotating frame effects,
body-specific response, and other inertial acceleration. It returns the one
`MotionFrame` for that body and tick.

That exact value governs:

- input interpretation;
- side/up/down/support directions;
- acceleration and free fall;
- jump, dash, blink, recoil, fast-fall, and flight directions;
- collision and one-way semantics;
- surface attachment, shedding, route steering, and crawler cornering;
- externally published contact and surface facts;
- every active movement policy.

A policy may not cache, author, snapshot, or reconstruct the current frame. A
frame change is not a policy change and cannot reset private policy state.

### 5. Directional values crossing the boundary are typed by frame

Untyped directional `Vec2` values are not accepted at the trusted boundary. The
input/controller seam distinguishes at least:

- raw device/screen axes;
- controlled-body-local locomotion intent;
- controlled-body-local aim intent;
- world-space scripted direction or impulse;
- reference-frame-relative direction;
- world/environment geometry.

Small newtypes or tagged enums are preferred over naming conventions. A
`ResolvedMotionIntent` (or equivalent) is built once from raw controller data and
the already-resolved `MotionFrame`, then passed unchanged to `step_motion`.

Screen-relative, strict body-relative, and assisted body-relative controls remain
controller policy. They are selected at this seam; they are not movement-model
parameters. Quick blink and precision aim may use different controller policies,
but their resulting vectors still carry explicit frame types.

### 6. Shared body state and policy-private state have explicit owners

The authoritative shared body state contains only facts whose meaning survives a
policy change, including world position, world velocity, facing, body dimensions
and active body mode, identity, controller ownership, health/resources, combat
state, and abilities.

Each policy owns its authored parameters and persistent private runtime state.
Examples:

- `AxisSwept`: coyote/jump buffers, wall/ledge state, dash/blink locomotion
  state, and axis-oriented contact caches when those facts are not genuinely
  shared;
- `SurfaceMomentum`: attached/airborne state, surface and route identity, arc
  length, signed tangential speed, depth lane, and junction/crossover state;
- `AdhesiveCrawler`: attached/detached state, current support normal/surface,
  corner transition state, and crawler-specific contact probes.

Historical cluster placement does not establish shared ownership. Axis-private
state must migrate out of global clusters when doing so makes the invariant
clearer.

Model parameter types may not contain current gravity direction, acceleration,
reference orientation, input-frame preference, or any other live environmental
fact. The historical `MovementTuning` aggregate must be decomposed rather than
copied wholesale into `AxisSwept`.

### 7. Policy transition semantics are deterministic and controller-independent

Applying a same-policy spec refreshes authored parameters and preserves all
private runtime state.

Applying a cross-policy spec preserves every shared body fact and initializes
only destination-private state:

- `AxisSwept` begins with empty policy-private support/contact caches; the same
  tick's ordinary collision phase computes current contacts from the unchanged
  pose. No coyote, jump-buffer, wall, ledge, dash, or blink state is imported
  from another policy.
- `SurfaceMomentum` begins `Airborne` on the unchanged pose and velocity. It may
  attach only through its normal same-tick contact/sweep rules. It may not search
  for the nearest route, teleport, or claim support from a stale shared flag.
- `AdhesiveCrawler` begins detached on the unchanged pose and velocity. It may
  acquire a support only through its normal same-tick contact rule; it may not
  nearest-surface snap during policy initialization.

A policy switch never changes the live environment frame. A frame change never
switches policy. The operation is independent of whether the current controller
is human, AI, possession, clone, replay, or RL.

### 8. Side paths have one explicit relationship to the kernel

- Moving-platform motion is contact/environment kinematics supplied to the
  kernel, not a second pose integrator.
- Water is typed medium/environment context (buoyancy, drag, acceleration
  response), not hidden gravity fields inside a model spec.
- Climbing, flight, dash, blink, ledge movement, and fast-fall are policy modes
  or typed ability commands consumed inside the selected policy. They do not
  move the body in outer systems.
- Knockback, recoil, explosions, and scripted pushes are typed world-space
  impulses/accelerations accumulated before the one kernel call.
- Body-mode changes alter shared body shape/mode through an explicit transition;
  collision reconciliation happens inside the kernel.
- Authored path/route steering produces typed intent or is an explicit
  `ScriptedKinematic` policy if it directly owns pose. No path system may remain
  an unclassified pose writer.

Any system that writes integrated-body position or velocity must either be the
kernel, initialize/restore shared state outside a movement tick, or be documented
as a non-integrated transform with an architecture guard proving that status.

### 9. Results and snapshots are policy-complete but frame-free

All policies publish one common result vocabulary for contacts, support/surface
facts, hazards, sweeps, reset requests, and movement events. Policy-specific
observations may be nested variants, but callers do not infer them from which
solver ran.

Snapshots preserve:

- the active policy identity;
- authored/runtime policy parameters when needed for deterministic continuation;
- policy-private state;
- authoritative shared body state.

Snapshots do not preserve the current `MotionFrame` as model state. After
restore, the frame is resolved from the live restored environment. Restoring a
body into a different active field changes the next tick's frame without erasing
valid policy-private state.

## Rejected alternatives

- **Absence means axis-swept.** Rejected because it makes migration and queries
  ambiguous and permits bodies to bypass policy invariants.
- **Outer ECS dispatch.** Rejected because controller/actor-specific systems can
  reconstruct different frames and publish incompatible results.
- **One giant parameter struct.** Rejected because it mixes model policy,
  abilities, controller preferences, and live environment state.
- **A trait hierarchy for its own sake.** Rejected. A small enum match is easier
  to audit and keeps state ownership visible.
- **Derive orientation from net acceleration every tick.** Rejected because zero
  acceleration and lateral/non-inertial acceleration need an independent basis.
- **Treat `surface_walker` as a boolean axis modifier.** Rejected because it is a
  whole pose integrator with distinct persistent state.
- **Nearest-surface initialization on policy swap.** Rejected because it hides
  teleportation and makes switching dependent on incidental nearby geometry.
- **Snapshot the frame inside the model.** Rejected because the environment owns
  the current frame and restore must respect the live field.

## Consequences

Positive:

- all controllers and body kinds exercise the same auditable movement law;
- arbitrary-angle and time-varying frames become testable by construction;
- policies can be swapped without reconstructing shared gameplay state;
- hidden third integrators and model-owned gravity become architecture errors;
- snapshots can continue deterministically while remaining responsive to the
  restored environment.

Costs:

- the migration is intentionally broad: input vectors, body clusters, actor
  integration, snapshots, demos, and tests all cross this boundary;
- some existing tests that directly call policy solvers must move behind the
  kernel or become engine-core policy-private tests;
- `surface_walker` must become an explicit policy rather than remain a tuning
  flag;
- environment resolution needs a real per-body aggregation seam rather than
  scattered `gravity.dir_at(...)` calls.

## Required validation before declaring the migration complete

Executable evidence must establish all of the following:

1. every integrated movable body has an explicit policy from spawn;
2. home, AI, possession, clone, and RL control reach the same public entry;
3. the environment resolves one frame per body tick;
4. input interpretation and policy execution receive the same frame value;
5. all policies are covariant under non-cardinal arbitrary-angle rotation;
6. zero acceleration retains an explicitly supplied orientation;
7. lateral acceleration does not rotate the supplied basis;
8. time-varying frame changes preserve private state;
9. same-policy parameter refresh preserves private state;
10. round-trip cross-policy switching preserves shared world-space state and
    initializes only destination-private state;
11. switching is controller-independent;
12. snapshot restore preserves policy/private state but uses the live frame;
13. no policy parameter type contains current-frame or controller-frame fields;
14. no production caller invokes a policy-specific whole-tick solver;
15. no integrated query treats a missing model as axis-swept;
16. `surface_walker` no longer bypasses the kernel;
17. ordinary Ambition movement and Sanic loop/momentum acceptance paths remain
    green.

Pure engine-core tests should prove physical, covariance, and transition
invariants. Narrow assembled tests should prove ECS wiring, controller routing,
spawn completeness, and snapshot integration. Architecture guards should reject
optional model queries, direct solver calls, model-owned frame fields, raw input
below the resolver, and integrated pose writers outside the kernel.

## Current implications for agents

- `docs/planning/engine/unified-movement-kernel.md` documents the shipped
  invariants + ownership map; this ADR is the source of truth for the
  architecture.
- Preserve the App-local character catalog. Movement policy authored by a
  character is resolved from the active experience's catalog, never a
  process-global Ambition roster.
- Preserve the later Sanic discrete-depth-lane and tangent-release fixes when
  moving the old `surface` module under `movement::surface_momentum`.
- Do not add compatibility aliases, optional `MotionModel`, or temporary outer
  dispatch to make an intermediate build easier.
- Compile early and repeatedly; rebaseline tests that pin obsolete architecture
  while retaining or strengthening their behavioral guarantee.
