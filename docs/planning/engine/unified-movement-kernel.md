# Frame-aware unified movement kernel

> **Binding decision:** [ADR 0024](../../adr/0024-frame-aware-unified-movement-kernel.md).
> This document is the migration ledger; where wording differs, the ADR wins.

## Non-negotiable law

Every movement tick is interpreted in the body's **current acceleration/reference
frame**.

The environment resolves one immutable `MotionFrame` for the body before movement
dispatch. It contains an independently supplied reference basis and the complete
world-space acceleration for the tick. That exact value reaches controller-intent
resolution, every movement policy, and every frame-relative limb.

A movement model must never:

- cache the current gravity/reference direction;
- reconstruct a private frame from its parameters;
- read raw screen-space controls;
- interpret world X/Y as body side/down;
- reset private state merely because the frame rotated or its acceleration
  magnitude changed.

`MotionFrame` carries both the complete world-space acceleration vector and an
orthonormal reference basis. Ordinary gravity may align them, but neither is
derived from the other at the trusted boundary. Zero acceleration retains the
environment-supplied orientation, and lateral/inertial acceleration does not
silently rotate that orientation.

This is the movement form of the project's principle of relativity: the laws are
written once in body-local coordinates and remain covariant when the environment,
body, or room frame rotates.

## Intent

Ambition has multiple legitimate movement policies. Axis-swept action-platformer
movement and surface momentum for slopes, loops, and high-speed routes are
currently the two customers. They are different algorithms, not different actor
pipelines.

The target is one small trusted kernel family:

- one explicit `MotionModel` on every integrated body;
- one `step_motion` entry for human, brain, RL, possessed, home, enemy, boss, and
  test controllers;
- one shared world/body/input/frame/timestep contract;
- sibling model implementations with private parameters and runtime state;
- one explicit state-preserving model-transition operation;
- no `None means axis swept` convention;
- no actor/home/demo-owned physics dispatch;
- no direct solver calls outside the kernel implementation.

## Drift-aware starting point

The rebased structural overlay is intentionally a migration foundation, not
completion evidence. It preserves the current App-local character catalog and the
later Sanic discrete-depth-lane and tangent-release fixes while establishing the
first shared boundary. The historical axis-only whole-tick APIs become
crate-private rather than a second public architecture:

- `surface_momentum` physically lives beside the axis-swept implementation under
  `ambition_engine_core::movement`;
- the old crate-root `surface` alias is removed and callers name the actual model;
- engine-owned `MotionModel` / `MotionModelSpec` values select the policy;
- `AxisSweptMotion` owns frame-independent `AxisSweptParams`;
- `SurfaceMomentumMotion` owns `MomentumParams`, ride state, and depth lane;
- `MovementTuning` remains only an authoring/control-boundary aggregate and is
  projected into `AxisSweptParams`; its gravity and input-mode fields do not enter
  the model;
- `MotionFrame` pairs net acceleration with the existing `AccelerationFrame`
  basis and is passed through `MotionStepContext`;
- home and ordinary actor integration both invoke `step_motion`;
- model refresh preserves same-model private state, while cross-model transition
  initializes only destination-private state;
- frame tests begin pinning zero-force orientation and arbitrary-angle covariance
  for both policies. They do not yet prove one environment resolution and one typed
  intent artifact end-to-end.

## Frame ownership

The three relevant facts have different owners and must remain separate:

1. **Environment** — current net acceleration / reference frame (`MotionFrame`).
2. **Controller seam** — raw input mapped once into controlled-body-local axes.
3. **Movement policy** — authored parameters and solver-private runtime state.

The current frame is not authored identity, model configuration, or snapshot
state. A snapshot stores the active model and the model-private state required to
continue it; after restore, the live environment supplies the current frame.

Likewise, input mapping preference is not an axis-swept parameter. The kernel
receives local `InputState`; screen/body-relative accommodation happens before the
trusted movement boundary using the same `AccelerationFrame` basis.

## Swap invariant

Changing movement policy preserves all model-independent facts:

- world position and world velocity;
- facing and body shape/mode;
- identity, controller ownership, health, abilities, and resources;
- environment contacts that are genuinely shared observations.

Only destination-private state is initialized. A same-model parameter refresh
must preserve private state such as surface identity, arc length, signed tangent
speed, depth lane, coyote/jump buffers, or other state that remains meaningful to
that model.

A frame change is not a model swap and must not reset either model.

## Remaining integration work

1. Replace the remaining phase-level axis test adapters with tests that enter via
   `step_motion`, then make the individual solver arms private.
2. Audit the current body clusters: retain only facts with genuinely shared
   semantics; move axis-only timers/contact state into `AxisSweptMotion` and keep
   surface-only facts in `SurfaceMomentumMotion`.
3. Separate locomotion policy from optional ability tuning currently bundled in
   `MovementTuning`; do not move that entire historical aggregate into the model.
4. Give the environment one authoritative per-body net-acceleration resolver.
   Room gravity, local fields, moving/non-inertial frames, and body-specific
   response compose there before `MotionFrame` construction.
5. Migrate the historical `surface_walker` integrator into the explicit
   `AdhesiveCrawler` policy selected by ADR 0024, then remove the tuning boolean and
   actor-owned pose-writing branch.
6. Define authored hydration and snapshot codecs at the model boundary. Never
   snapshot `MotionFrame` as model state.
7. Add architecture guards forbidding optional `MotionModel`, direct policy-step
   calls, model-owned gravity/reference fields, and raw screen input below the
   kernel seam.
8. Expand covariance evidence from cardinal rotations to arbitrary-angle and
   time-varying frames, including model swaps while the frame rotates.

## Acceptance evidence

The completed integration must prove:

- every integrated body carries an explicit model;
- all controller/body kinds reach the same public movement entry;
- both policies consume the same frame value for a tick;
- rotating the world, acceleration, velocity, geometry, and local controls rotates
  the result without changing body-local behavior;
- changing acceleration magnitude does not rotate the basis or reset model state;
- rotating/changing the frame does not count as a model refresh;
- axis → momentum → axis and momentum → axis → momentum preserve shared
  world-space state;
- snapshots restore model parameters/private state but resolve the frame from the
  restored body's live environment;
- no model parameter type contains current gravity direction or acceleration.
