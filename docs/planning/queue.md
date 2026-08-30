# The queue — live execution order

This file is the executable current-work ledger. It is intentionally
self-replenishing: when the listed rows are exhausted, verify the highest-value
item in [`tracks.md`](tracks.md) against HEAD, promote it here, and continue.

There is **one row shape**:

```text
- ▢ **ID — current problem.** Current evidence, next action, acceptance.
```

`▢` appears only on executable rows. Completed investigations do not stay here;
use git history. A focused plan owns technical design and should be linked rather
than copied into this file.

Before implementing a row, re-check the named gap against current source/tests.
Direct new maintainer observations outrank this ordering when they are
reproducible.

**Reviewed baseline:** `4e5f59cf753a62105cbc9fd53aa9697d337d0eed`.

## Recent structural receipt

✔ **D-SESSION-OWNERSHIP — cross-game rollback health contamination.** Fixed by
`26ec7b19`: rollback authority is owned by `SessionScopeId`, health carries only
across timelines of that same gameplay session, foreign-session confirmation is
`Unavailable`, and session-mirrored resources are re-established on activation.
Guarded by shell lifecycle and session-ownership tests. Durable rule: ADR 0027.

The one unresolved developer-policy choice from that work is in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §37.

## Current execution order

- ▢ **D-POLICY-1 — return `ambition_workspace_policy` to green after recent
  carves.** The 2026-08-30 session-ownership work repaired two stale scanners and
  exposed a reported set of pre-existing policy failures across orientation,
  platformer facade re-exports, module size, clock reset, manifest allowlists,
  pose writes and velocity writes. Triage each against the current durable
  boundary; fix code or update a policy only when the ownership rule itself has
  changed. **Do not bulk-waive the suite to make it green.** Acceptance: the
  policy crate passes and each changed policy still points at a current durable
  source of truth.

- ▢ **D-SIM-LOCAL — remove authoritative non-rewinding edge/history state.** The
  current review still identifies Mary-O `follow_the_active_room` memory that can
  treat a restored historic room as a new transition and quest/room-visit edge
  detection that deliberately does not rewind. Classify each memory as
  authoritative, derived, or presentation/diagnostic; registered authoritative
  history must rewind, and derived history must be reconstructable. Acceptance:
  rollback through the relevant transition/visit cannot apply a future-only
  mutation after restore. Owner:
  [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).

- ▢ **D-SIM-SELECT — close the remaining deterministic selection/composition
  sites.** Current review evidence names projectile-victim ties, possession
  candidates and pickup-magnet ownership. Use stable semantic keys for true
  selection. If several peers compose into one result, first state whether the
  operation is commutative; a stable sort is not enough when precedence changes
  the physics. Acceptance: reversing ECS insertion/query order does not change
  the selected/composed authoritative result. Owner:
  [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).

- ▢ **D-PORTAL-POLICY — remove process-global portal mapping policy from live
  simulation.** `ambition_platformer2d_shared_tangle::math` still stores the
  active portal mapping convention in `PORTAL_MAP_ROTATION: AtomicBool`, while
  the pure map functions already accept an explicit convention. Move the
  effective mapping/facing policy into provider/session authority and thread it
  through live portal consumers. Acceptance: two Apps/providers in one process
  can use different conventions without process-order contamination, and
  synchronized session rules determine identical portal behavior. Owner:
  [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).

- ▢ **D199 — replace the centre-line anti-tunnelling ray with policy-aware swept
  body geometry.** The current ray tests a body's centre line against solids and
  can miss collisions a finite AABB should hit; it also bypasses collision
  policy distinctions. Implement swept AABB/Minkowski-equivalent continuous
  collision using the same policy authority as ordinary movement rather than a
  second geometry rule. Guard with edge/corner cases that the centre-line probe
  misses and with a contrast case for intentionally passable geometry.

- ▢ **D-CONTROL-ITEM — make held ranged/custom item actions per driven body.**
  Review evidence still shows held ranged shots attributed through the primary
  slot and custom held-item abilities using singular `ControlledSubject` while
  generic pickup/throw/fire already iterate driven bodies. Converge on one
  per-body request/ownership road; prefer the shared projectile request seam over
  a parallel held-shot simulation. Acceptance: two independently driven bodies
  can use their held/ranged actions in the same tick with correct owner/seat/body
  attribution. Owner:
  [`engine/controlled-character-actor-kernel.md`](engine/controlled-character-actor-kernel.md).

- ▢ **D-FIGHTER-L6 — diagnose the confirmed rollout regression with a decision
  trace, not another sweep.** The controlled A/B already established the signal:
  rollout-enabled level 6 fails the cited recovery scenario 45/45 while disabled
  succeeds 45/45; level 1 is unaffected and RecoveryLens did not fix it. Trace
  one fixed seed through option generation, rollout vetoes, suicidal-movement /
  support recovery reasoning, least-bad selection and final choice. Fix the
  first wrong authority/decision exposed by the trace. Owner:
  [`engine/fighter-brain.md`](engine/fighter-brain.md).

- ▢ **D72 — continue Super Smash Siblings as a product/engine customer from the
  current parity inventory.** Do not resurrect the historical fun-push campaign.
  Re-read [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md)
  and current maintainer observations before choosing the next slice. Prefer
  mechanics/readability/control defects that expose reusable engine seams over
  broad polish. Explicitly keep already-settled genre decisions and shipped
  mechanics from being reimplemented.

- ▢ **D-RASTER-3 — split the weak-GPU improvement between framebuffer scale and
  MSAA.** The valid matched result is **51.045 ms -> 20.101 ms p50, about 2.54x**;
  both DPI/framebuffer cap and MSAA changed together. Run an interleaved A/B on
  real weak GPU hardware with the independent `AMBITION_MAX_SCALE_FACTOR` and
  `AMBITION_MSAA` knobs, multiple reps per arm, holding build/features/profile
  constant. Do not substitute lavapipe/software rendering. Owner:
  [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

- ▢ **D33 — continue actor-monolith decomposition by coherent ownership.** Pick a
  carve that removes a real authority/dependency edge from the residual actor
  kernel, moves registration/tests with the domain, and improves capability or
  compile/test isolation. Do not carve by LOC and do not promise frame-time
  improvement without a measurement. Owner:
  [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md).

- ▢ **D166 — make the character-authoring boundary load-bearing where a real
  character still bypasses it.** Prepared character definitions are already
  immutable and the first Smash fighter facet exists. Re-measure the current
  residuals before migrating another field. The startup-reach proxy is a
  maintainer decision (§35), not an excuse to widen generic character data.
  Owner:
  [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

- ▢ **D129 — repair player-visible sprite clipping through authored geometry,
  using the existing build-time guard.** The historical "52 of 196" count is a
  stale snapshot; the later full render census also moved as art was repaired.
  Re-run the current target, start with player-visible/selectable characters that
  still fail, and fix the authored canvas/pose/geometry rather than weakening the
  guard. Do not infer a roster-wide scale rule from one character's repair.

- ▢ **D-INPUT-RECORDER — remove quadratic rollback copying from
  `InputStreamRecorder`.** The recorder owns a growing history `Vec` that is
  cloned into rollback state, making later frames copy the whole prior stream.
  Keep finalized recording outside speculative rollback and rewind only the
  cursor/unconfirmed tail needed to reproduce state. Acceptance: equivalent
  recorded output across rewind with bounded per-frame snapshot growth. Owner:
  [`engine/netcode.md`](engine/netcode.md).

- ▢ **D-TRAP-HOLD — make `UntilPressedAgain` use the semantic timeline-hold
  action set it claims.** Current behavior describes "any action press except
  movement/jump/dash" but checks only Attack/Special. Model the release input as
  a reusable hold/timeline semantic with Smash charge as one customer rather
  than adding another hard-coded verb list. Also repair the stale `trap_probe`
  comments that still describe the withdrawn self-release diagnosis.

## External measurements / human-gated work

These are live but should not cause an autonomous agent to invent data or a
product ruling.

- **Switch Pro outer range:** run `Shift+F6` on both machines, push the controller
  to each extreme/corner and compare peak axis magnitude. Only then decide
  whether shared outer saturation is needed.
- **Character/product decisions:** see
  [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), including
  the proof-pulse lifetime, character heights, fighter reach/tumble policy,
  ranged-recharge presentation, persistent foreign-room actor placement and
  dormant windbox/armor customers.
- **Rendered external-consumer/platform checks:** keep in
  [`tracks.md`](tracks.md) until the necessary GPU/toolchain is available; do not
  report host-prerequisite absence as an engine defect.

## Replenishment rule

When these rows thin out:

1. inspect [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
   for direct untriaged reports;
2. inspect [`tracks.md`](tracks.md) in its stated replenishment order;
3. re-measure the candidate against HEAD;
4. promote one concrete executable slice here with a focused owner and
   acceptance criterion;
5. keep going.

Do not recreate a staffing table or a second review-status ledger. If a task is
not executable now, it belongs in tracks or the maintainer-decision file rather
than hidden inside a closed queue narrative.
