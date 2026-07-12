# Surface joint zero-speed support ambiguity

## Prompt shape

A surface-momentum body can traverse a ramp or loop at speed, but if it reaches
exactly zero tangential velocity on a polyline vertex it appears stuck: jump,
crouch, and walking off stop working even though the body is visibly touching a
valid supporting branch. Diagnose and fix the engine invariant rather than
reshaping the level around the vertex.

## Hidden cause

`SurfaceMotion::Riding` stores `(surface, arc_length, v_t)`. At an exact joint,
arc length names two adjacent frames. `SurfaceChain::frame_at` resolves the tie
to the pre-join segment because of its `s <= len` walk. If that segment is
wall-like while the post-join segment is a floor, the low-speed stick rule sees
no pressing gravity load and sheds the body to `Airborne`. Grounded affordances
then disappear together, which makes the symptom look like several unrelated
input bugs.

## Transferable invariant

A resting joint needs a deterministic adjacent-frame resolver. Choose the
branch from current tangential travel when moving, otherwise the adjacent
segment with the strongest gravity press, and use held along-surface intent only
when support is tied. Nudge arc length by a representable ULP-scaled amount so
subsequent frames no longer repeat the ambiguity. When the body leaves a joint,
carry the exact departure frame from the branch being left; recomputing
`frame_at` at the shared arc coordinate can substitute the wrong tangent. Open
endpoints are not joints and keep the ordinary launch/fall policy.

## Poison tests

Construct an authored wall-to-floor chain and place a zero-speed rider exactly
at the shared vertex while its center is on the floor side. The test must prove
all of the following:

- idle remains riding on the floor segment;
- positive run advances onto the floor;
- jump leaves along the floor normal;
- intent toward the unsupported wall branch sheds rather than pins.

A level-only smoothing test is insufficient: dense geometry can hide the bug
without repairing the zero-speed state.
