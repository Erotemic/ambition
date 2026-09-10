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

⛔⛔ **THIS SECTION NAMED A1 UNTIL 2026-09-10, AND A1 WAS ALREADY FINISHED. IT SENT
THREE AGENTS AT COMPLETED WORK IN ONE DAY.**

**A1 is DONE.** Audited row by row, not inferred from the packets: the
[checkpoint protocol](engine/checkpoint-restoration-protocol.md)'s executable
acceptance matrix holds **18 rows, and all 18 carry a named witness** in the
audit table beneath it. A1a's admission latch, A1b's move of restoration into
session, and A1c's single immutable checkpoint are each closed there. The
frontier's readiness map marks A1a/b/c DONE. Subcommit 5's targets are gone —
`ResetToCheckpoint` has no raw restoration readers left, only writers and
registration.

⚠ **One item stays open and the protocol scopes it itself:** the terminal
outcome has no presentation consumer, *"when one is wanted"*. **That is a
deferral, not a defect, and it is not an architectural action.**

⇒ **THERE IS NO SINGLE NEXT ARCHITECTURAL ACTION NAMED HERE ANY MORE, and that is
deliberate.** A section that names one goes stale the moment that one lands, and
this one stayed wrong long enough to misdirect three agents on the day it was
noticed. **The open packets and their state are listed below; read those.**

⚠ **AND THE COUNT ABOVE IS 18, NOT 17.** The agent who audited the rows reported
17. Both a count of the matrix and a count of the audit table give 18. **The
audit's finding stands; its arithmetic did not, and a number that crosses a
document boundary travels with the method that produced it.**

The [projectile protocol](engine/projectile-contact-protocol.md) makes A2's
geometry sample, finite-shape obstruction, compound contacts and targeted delivery
explicit. A3 retains the valid world-to-construction lowering move. The
[authored-technique protocol](engine/authored-technique-admission.md) specifies
A11/A12's installed support, exhaustive reference validation, checked acyclic
flows, move-clock semantics and explicit prepared-revision activation.

⚠ **THIS PARAGRAPH SAID "these are implementation targets, not landed fixes"
UNTIL 2026-09-10, AND BY THEN IT WAS FALSE FOR THREE OF THE FOUR.** A protocol
document describing a target reads identically to one describing shipped
behaviour, so the distinction has to be maintained by hand here — and a blanket
sentence covering four packets goes stale the moment any one of them lands.
Stated per packet instead:

- **A2** — the RESOLVER half is met: one real swept projectile leg, exact
  `total_cmp` ordering, the selected world collider carried into physical
  response, and an identified victim ordered ahead of an unidentified one. ⛔
  **The CONSTRUCTION-IDENTITY row is NOT met and this line used to say
  otherwise.** The protocol calls a missing deterministic identity a
  construction failure rather than a sort fallback, and construction still
  permits one. ⚠ **The specific example a review supplied — the cut-rope victory
  NPC — was REFUTED**: it carries `FeatureId` + `BodyKinematics`, which is
  exactly the query `ensure_sim_id` serves, so it is identified a tick later.
  **What is open is the general invariant, not that case.** The COMPOUND SOLID
  row remains deferred to Q96/A5 and is a maintainer ruling rather than
  unfinished work.
- **A3** — done; the only residual is a file move that removes no edge.
- **A11** — REOPENED and **FULLY re-closed** 2026-09-10 after a review of the 125
  commits following `6a692b6`. ⛔⛤ **The shipped game had been closing its
  admission barrier UNCHECKED**: Bevy's runner does `finish()` before the first
  `update()`, `PreStartup` lives inside that update, and the unchecked backstop
  therefore won in every app reaching `App::run` — while every guard agreed,
  because they all drive `update()` by hand, which never runs `finish`. Fixed by
  ONE DECLARED AUTHORITY, and the witness that says so differs from its green
  sibling by three calls. **Transitive withholding is fixed too**: the one-pass
  filter published a summoner whose beast it had just refused.
  ✅ **CLOSED 2026-09-10 by `4fe4a1a27`, and this line said
  otherwise for half a day.** `admit_and_finalize_cast` now takes
  `support: &TechniqueSupport`, **not `Option<&_>`** — the comment on that
  parameter says *"NOT `Option`, AND THAT WAS THE LAST A11 BLOCKER."*
  `TechniqueSupport::admit_at` opens with `self.offers.get(&effect.key)` and
  returns `TechniqueRefusal::Unknown` on a miss, so an EMPTY table refuses every
  native effect. ⇒ **A composition that installs no technique handlers does
  not have an UNKNOWN support set. It has the EMPTY one.** Both production
  empty-table roads are deliberate and documented: the `finish()` backstop, which
  stands down on `ChecksAuthoredEffectsAtTheBarrier`, and the public
  `close_preparation_barrier` for a host that installs none.
  ⚠ **ALL THREE A11 BLOCKERS ARE CLOSED. A11 HAS NO OPEN BLOCKER.**
- **A12** — ⛔ **PROPAGATION LANDED (`f9baa86e8`); OCCURRENCE IDENTITY DID NOT.**
  This line read *"NOT fixed"*, then *"LANDED"*, and both were wrong. What landed
  closes the DIRECT-REPLACEMENT case, where `succeeding(Some(prev))` genuinely
  increments. ⛔ **`MovePlayback::instance` is a WITHIN-CHAIN ORDINAL, not a
  body-local identity**: `new_at` sets it to `0` and `succeeding(None)` maps back
  to `0`, and the playback component is REMOVED when a move ends — so a move
  starting after an idle gap carries `0` again, and a shot stamped `0` by an
  earlier move is credited to it. ⚠ Concretely reachable: `officer_the_draw`
  fires at 0.348s and ends at 0.696s while its projectile lives up to 2.4s.
  ⛔ **Reflection** re-owns a shot to the interceptor (`intercept.rs:82`) and
  leaves `FiredByMoveInstance` untouched, pairing a new owner with the old
  shooter's stamp. ⛔ **`None` still credits the current move**, so clearing the
  stamp on reflection is the same defect with an extra step. A12b's
  prepared-revision items remain open and always said so. A verdict carrying
  `attacker_move_instance: None` WAS credited to whatever move the fighter was
  playing at the time, so a projectile launched by move A and landing during
  move B marked B connected — the late-feedback defect A12 exists to eliminate. The
  predicate's own comment defends `None` for hazards and contact attrition and
  never addresses the projectile. **Both halves were needed and both
  landed:** the instance now travels `MovePlayback::instance` →
  `MoveEventMessage::move_instance` → `RangedCommitment::CommittedMove`
  → `ProjectileSpawnRequest` → `FiredByMoveInstance` on the shot
  → the damage result, and the
  predicate then requires the claim. ⚠ **The value is ABSENT, not zero**,
  for a shot no move fired — a `0` would name a first use that never
  played. `GGRS_ROLLBACK_SCHEMA_VERSION` 178 → 179.
- **A7** — RECLASSIFIED from an occurrence-ownership completion to a
  COMPONENT-CONSTRUCTION SEAL. `GroundItem` is sealed and that is real; *"seven
  minting authorities became one"* is not, and callers still mint identity,
  custody, provenance and attempt state themselves. ⇒ Calling a centralized
  component constructor does not transfer authority over the OCCURRENCE.
  Q97's policy half is still Jon's.
- **A9** — open, and its next question is which of the 49 crates a minimum
  profile has a RIGHT to expect for the crates the frontier does not name.

The frontier records prerequisites and holds; the queue selects priority rather
than a predicted SCC trajectory.

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

⚠ **RE-MEASURED 2026-09-10: it is 48, at `939d6aaa5`.** The 51 is the
`300004d601af1e633cfaee969f079cf9bb368ca8` baseline. Three edges closed
between the two, all on 2026-09-09: the render path through host, five dead
dependency declarations, and the map capability. ⇒ Reproduce with
`cargo tree -e normal --no-default-features -p ambition_platformer2d`, count the
unique `ambition_*` names (49) and subtract the facade itself (48).
⛔ **THE UNIT IS THE TRAP.** 49 counts the facade, 48 does not, and this page's
51 is an *other-packages* count. A number that cannot say which it is cannot be
quoted. See `scripts/measure_minimum_profile_parentage.py`.

## Correctness and validation front

[Findings F1-F9](engine/architecture-review-findings.md) distinguish conditional
startup retry loss, divergent boss/projectile geometry, obstruction ordering,
inert authored fields, nonoptional render reachability, construction publication
limits, unwired technique validation, flow admission bounds and raw checkpoint
reset mutation outside room admission. Each has a counterexample or verification
task and a responsible packet. Do not represent them as reproduced Rust failures
from this review.

[Repository history](repository-history.md) explains the intentional, repeatable
Git epochs and recovery from cold storage. A locally unresolved old commit is
an evidence-availability result, not proof that its history was lost. Remote
archive metadata was inspected; full reconstruction was not executed here.

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
