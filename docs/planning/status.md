# Planning status

Live execution is in [queue.md](queue.md). Product rulings remain in
[maintainer-decisions.md](maintainer-decisions.md); unanswered choices remain in
[awaiting-maintainer-decision.md](awaiting-maintainer-decision.md).

**Architecture review baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`,
2026-09-08 committed source archive. Source inspection and Python architecture
checks were available; Rust compilation, gameplay, GPU and network execution
were not. The [coverage receipt](engine/architecture-review-coverage.md) separates
those evidence classes.

## Architecture posture

The [reassessment](engine/architecture-reassessment.md) replaces the mandatory
P2-P5 SCC sequence with ownership-based packets. Construction inversion,
backend-neutral rollback registration, published phase sets and the corrected
actor-spawn boundary remain supported. None establishes that all state, lifetime
or capability boundaries have converged.

The reusable substrate coexists with a distributed integration kernel across
actor-monolith, combat, characters, core and runtime. The
[responsibility map](engine/architecture-responsibility-map.md) distinguishes
logical authorities from present crate locations. A new crate is not an exit
criterion by itself.

## Next architectural action

**A1: checkpoint restoration ownership.** First characterize/fix startup's
admission latch; then move restoration, progress and installation into session.
Leave healing and checkpoint capture at the rest-point interaction. Do not move
room/session lifecycle vocabulary into `shared_tangle` to remove an import.

A2 addresses projectile geometry/contact correctness before removing feature
knowledge. A3 retains the valid world-to-construction lowering move. A11/A12
address disconnected authored parameter validation and flow representation
bounds. The frontier records exact prerequisites and holds; the queue selects
priority rather than a predicted SCC trajectory.

## Measured shape, not architectural acceptance

The module-path instrument reports a nine-module SCC (abilities, construction,
control, features, items, projectile, session, shrine, world) and the two-module
assets/character_sprites SCC. It excludes several test forms but is a textual
heuristic, not a Rust semantic dependency graph.

The workspace inventory has 79 packages and 679,785 physical Rust lines under
package `src` directories, including comments and tests. Of those, 98,464 are in
the actor monolith. Foreign-ordering/installation instruments report 0 capability
private orderings, 73 composition private orderings and 174 foreign system
installations; installation classification reports 3 reducible and 38 irreducible
blocks. Reproduce counts before quoting them for another revision.

The facade's nonoptional internal dependency traversal reaches 51 other workspace
packages even without selecting optional dependencies. Render is reachable through
host. That is a source-only lower bound, not Cargo's resolved feature closure,
link size, memory consumption or a runtime measurement. The SDK needs independently
verified minimal profiles rather than an opt-out claim based only on plugin flags.

## Correctness and validation front

[Findings F1-F8](engine/architecture-review-findings.md) distinguish conditional
startup retry loss, divergent boss/projectile geometry, obstruction ordering,
inert authored fields, nonoptional render reachability, construction publication
limits, unwired technique validation and flow admission bounds. Each has a
counterexample or verification task and a responsible packet. Do not represent
them as reproduced Rust failures from this review.

The already-landed mark-attribution, mark-stock lifetime, fuse timing, mark-clock,
body-owned portal publication, hit-flash release, body-clock public set,
map-visited lifetime, installer preflight, mount/possession arbitration and Mary-O
render-basis repairs remain closed absent new contrary evidence. This review does
not reopen them from stale reports.

## Standing constraints

Body-owned presentation follows simulation/read model, finalized drawable,
portal publication and pane composition in that order. Performance claims require
the intended scenario, cache state and hardware. Quality tier, device residency,
authored world dimensions and trim placement remain distinct contracts.

Fighter-brain policy stays generic over an authored move menu. Game product choices
are not resolved by a package move. No recommendation in this review is a new
maintainer ruling. Completed work leaves the live queue; Git retains its history.
