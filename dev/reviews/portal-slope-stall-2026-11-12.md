# Portal slope crossing stall

## Evidence

The desktop bundle `desktop-timeline-run-20261112T024029Z` uses source
`e1737f19f030`, the `profiling` build profile and no Tracy. Channels c130/c131
are the lab's slope-to-wall pair, station E. Frames repeatedly take about
2.4 seconds. Each frame runs 15 simulation ticks. The final census attributes
about 151 ms per tick to integration and 7 ms to `PlayerSimulation`.

F8 writes five gameplay traces. The last is
`debug_traces/ambition_gameplay_trace_1794451274-862137876-000004_20769d02h41m14s.json`.
At tick 1147, the body crosses from the slope to `(3450.9165, 891.1381)`.
Its velocity is `(-710.90656, -408.77127)`. Subsequent frames retain both values.
The real-app reproducer starts before that crossing at `(2944.9165, 861.8095)`
with velocity `(0, 707.55005)`. The crossing succeeds. The next movement step
refuses with `WorkLimit`, path `[]`, link `31`.

## Cause and correction

The refusal comes from the curved collider search during physical righting.
The far-side piece queries the slope segment from `(2832, 960)` to `(3032, 845)`.
Both segment endpoints lie outside the piece's fixed portal plane, by about
0.000042 world units. The exact clipped region is empty. The conservative
curved envelopes still overlap across that small gap, so repeated subdivision
exhausts the 16,384-interval budget. Every movement retry repeats this work.

The existing world-plane separation check handles polygons but excludes line
colliders. Apply that same check to segments and points before curved contact.
A polygon needs nonzero clipped area. A segment or point needs a nonempty
clipped region, so a boundary contact remains a candidate. A cut applies only
when its rate matches the collider's displacement in the query interval.
Rotating colliders keep the existing curved query. Non-finite collider input
still refuses before this check.

The correction keeps the canonical material solver, exact portal cuts, and
typed refusals. It does not raise a work budget or turn a refused computation
into an accepted miss. The earlier station J stop has no matching gameplay
trace and remains unverified.

## Verification

The 101 material-cast tests pass. The non-finite collider guard now includes
both segments and polygons. The real-app station E regression checks the
crossing, no movement refusal, and progress beyond the recorded stuck pose
under rollback, without a reset. Both lab tests pass. The assembled 80/40
partial-contact, reverse-travel and deterministic replay check also passes.
The broader suite is not run. Logs and commands are under
`target/portal-e-stall/`.
A new desktop run is needed to measure frame time at station E after the fix.

## Reverse crossing follow-up

The next F8 trace is
`debug_traces/ambition_gameplay_trace_1794453749-348163725-000000_20769d03h22m29s.json`.
The body walks right from `(3444.092, 921.99994)` into the wall aperture.
At tick 1726 it returns to the slope at `(2968.5095, 880.05524)` with velocity
`(-134.5872, -234.0647)`. The next 42 recorded ticks keep that pose and velocity.
The real-app reproducer matches this crossing and refuses the next tick with
`WorkLimit`, path `[]`, and no link key.

The corresponding bundle is `desktop-timeline-run-20261112T031559Z`, source
`9779af7b0650`, with the `profile` feature and Tracy enabled. The final census
reports about 416 ms per tick in integration. Its last frame takes 6.449 s.
These traced times are not comparable to untraced frame times as a controlled
performance estimate. They confirm that integration owns this stall.

This query has a real contact region. It cannot use the disjoint-line filter
from the forward correction. The curved search rejects one contact component
and must prove that the same overlap persists before it can skip that time
interval. Its old proof bounds a collider point and portal cut separately in
the rotating body chart. That loses their shared physical motion and forces
very small time subdivisions along the slope.

The correction reports whether the existing world-plane set contains every
active cut with fixed eligibility. For a collider with no angular motion,
its material point and each such plane have an affine relative distance.
Use that distance, with a bound for numerical rounding, to prove containment
in the cuts. Keep the existing body-containment proof. An incomplete plane
set, changing eligibility, or a rotating collider retains the existing
conservative proof. No contact component is skipped without a proof that its
overlap continues. Refusal and contact-admission rules remain unchanged.

The reverse regression drives the crossing under rollback, checks for refusals,
and requires the body to move clear without resetting. All 101 material-cast
tests, all three lab tests, and the 80/40 partial-contact, reverse-travel and
bit-identical replay guard pass on the final implementation. The broader suite
is not run. Logs and commands are under `target/portal-e-reverse-final/`.
Desktop confirmation at E in both directions and the earlier station J stop
remain open.
