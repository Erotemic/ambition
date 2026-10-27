# Portal clipping at world coordinates

## Task

A body crosses a rotating portal. Most frames take the usual time, but some
frames stop for several seconds. Find the cause without reducing the portal
piece budget or adding a separate path for fixed portals.

## Invariant

Two opposite half planes through one point share a line, which has no area.
Clipping a volume to both planes must produce no piece. This must hold at
every position and angle. A cut must not make a new fragment on its own edge.

## Cause and repair

Single precision intersections at world coordinates can round the two cuts
to different points. An area sum in world coordinates also loses precision.
The result can be a thin false polygon. The partition can cut it again and
add more bounds. The work limit stops further cuts. The extra bounds increase
the cost of clipping and removal of redundant bounds.

Keep intersections in double precision until the area test. Sum triangle
areas relative to one vertex. Convert the final polygon to the public vector
type after this test. Changing only the area sum does not fix rounded cuts.

## Evidence and validation

On 2026-10-27, repeated crossings of portal_lab's spinning pair with the dev
profile, at Ultra, 320x180, on llvmpipe, gave frames of 6.3, 7.2, 8.3 and
12.3 seconds with the original clipping code. The corrected run completed 840 frames, including
53 transfers. After startup, its largest recorded frame was 53 ms. These
times describe the VM, not a hardware GPU.

The regression tests cover opposite planes at all integer degree angles,
translated volumes, both map conventions, and both piece budgets. The carry
test moves and rotates a pair while the body changes ends. It requires two
pieces and no increase in images, materials or meshes.

Run the core `frame::thread::tests` and presentation `carry::tests`. Use the
full portal lab for timing: a small isolated pair can produce false pieces
without a large timing change.
