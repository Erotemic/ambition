# A guarantee documented on a FIELD, enforced inside ONE branch

**Tags:** `architecture-invariant`, `movement`, `relativity`, `policy-switch`,
`latent-defect`

## What happened

`MovementTuning::flight_invariant_speed` documented a hard promise on its own
doc comment: *"the flight limb accelerates in proper-velocity space and converts
back to a coordinate velocity, guaranteeing a subluminal result."* The flight
limb is a three-way policy switch:

```rust
let (mut new_run, mut new_descend) = if tuning.flight.direct_velocity {
    (target_run, target_descend)                 // verbatim command
} else if let Some(invariant_speed) = tuning.flight.invariant_speed {
    let c = invariant_speed.abs().max(f32::EPSILON);
    let terminal = tuning.flight.terminal_speed.abs().min(c * (1.0 - 1.0e-5));
    /* ... proper-velocity integration ... */     // ← the promise lives HERE
} else {
    /* ... accel/drag ... */
};
new_run = new_run.clamp(-tuning.flight.terminal_speed, tuning.flight.terminal_speed);
```

`direct_velocity` is tested **before** `invariant_speed`, so a direct-velocity
body skipped the branch entirely — and the two clamps that every branch's output
passes through read the **raw authored terminal**. With the engine default
terminal (760) and TwinTrack's invariant (600), a direct-velocity flyer came out
at exactly `760` against a `c` of `600`.

The correctly-bounded value already existed **three lines from the clamp that
ignored it**: `terminal`, a `let` scoped to the one branch that did not need a
clamp at all.

## The transferable invariant

**A guarantee documented on a data field is a postcondition on the value's EXIT
from the system, not a property of the branch that happens to compute it.** Put
the bound where every policy's output converges — for a policy switch, that is
after the switch, never inside an arm.

Two tells that generalize:

- **A correct value bound as a `let` inside one arm of a policy switch is a
  design smell**, not a local optimisation. If it encodes a law rather than an
  arm-local step, it belongs to the type that owns the fields it derives from
  (here `FlightTuning::coordinate_speed_cap`) so the arms and the enforcement
  point read one answer. One question, two answers, again.
- **Branch ORDER silently narrows a documented scope.** `if A { } else if B { }`
  means the promise attached to `B` does not hold when `A`. Nothing in the type,
  the doc comment, or the test suite says so; the reader has to notice that two
  independently-authored knobs are consumed by mutually exclusive arms.

## Why the tests did not catch it

The suite deliberately exercised `direct_velocity = true` **with**
`invariant_speed = Some(600.0)` — and passed, because it also authored a terminal
of `540`, *below* c. The combination was covered; the only configuration that
breaks the promise was not. A guard written from the motivating example tests the
example, not the invariant. The probe that found this differs from the existing
test in exactly one authored number.

## Where it is enforced now

`crates/ambition_platformer2d_core/src/movement/integration.rs` —
`integrate_flight_clusters` binds `tuning.flight.coordinate_speed_cap()` once,
above the switch; the branch and both clamps read it.
Test: `an_invariant_speed_bounds_every_flight_policy_not_just_the_relativistic_one`.
