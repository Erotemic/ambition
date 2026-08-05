# TwinTrack

> **Status (2026-08-04): candidate complete SR proof of concept; compile and visible feel validation remain.**

TwinTrack is the 2D executable acceptance game for Ambition's first relativity
slice. The player leaves a laboratory, reaches a far turnaround beacon, and
returns. A stationary laboratory clock and the player clock begin and reunite at
the same events. The player clock, animation, and any other existing
`ProperTimeScale` consumers advance by the traveler's proper time.

## Consumes

- the normal provider/session lifecycle;
- ordinary platformer movement and collision;
- `ambition_relativity` Minkowski mathematics;
- `ambition_relativity2d` clock integration and read model;
- declared HUD and ordinary presentation.

## Owns

- the Minkowski laboratory's authored invariant speed (`c = 600` world units/s);
- a traveler tuned to `0.9c` maximum run speed;
- the outbound/return/reunion rules;
- the laboratory and turnaround geometry;
- HUD wording and result presentation.

## Acceptance

- the provider runs standalone and in `ambition_app`;
- two clocks are present under one session-scoped Minkowski provider;
- a forced `0.9c` worldline accumulates proper time at approximately
  `sqrt(1 - 0.9^2)` times the laboratory rate;
- a forced outbound/return path reaches a reunion result;
- leaving the session removes the spacetime provider;
- no relativity system changes body velocity.

TwinTrack is not Slower Light. It contains no optical observer transform,
retarded image, variable light-speed zone, or 3D world. It grounds the local
clock and observer mathematics those later systems need.
