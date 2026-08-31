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

## Recent structural receipts

✔ **D-RECONSTITUTION — the same-room replay was a second room constructor.**
`reset_ecs_room_features` mutated twelve families of surviving entity back
toward a presumed spawn state through a hand-kept list. Measured divergence: a
replayed enemy came back facing the wrong way, 34.6px from where a fresh entry
puts it. A replay now records a lifecycle intent for the ACTIVE room and the
room-transition road rebuilds it, so new session / transition / replay /
new-game reset differ only in target room, retire filter and durable-fact
policy. Two defects fell out: a boss's persisted defeat was re-derived from its
corpse every frame (so a retraction was overwritten on the next), and a
harness-staged scenario actor had no reconstruction record (it survived a reset
and not a door). Guarded by `canonical_reconstitution.rs`. Owner:
[`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).

✔ **D-SESSION-OWNERSHIP — cross-game rollback health contamination.** Fixed by
`26ec7b19`: rollback authority is owned by `SessionScopeId`, health carries only
across timelines of that same gameplay session, foreign-session confirmation is
`Unavailable`, and session-mirrored resources are re-established on activation.
Guarded by shell lifecycle and session-ownership tests. Durable rule: ADR 0027.

- ▢ **D-RESTORE-INTERIM — prove the pre-correction population is harmless, or
  remove it.** A save load builds its first room with no occurrence continuity
  and corrects it afterwards; the acceptance suite establishes the two roads
  agree on the FINAL population, not that the interim one is safe to exist or to
  run. No gameplay gate is tied to `SaveRestored`, and the boss-defeat record is
  a live example of state re-deriving over a restored fact inside such a window.
  Take one of: a genuine `load_save_at_startup` test showing gameplay cannot
  observe or mutate the pre-correction population; a gate on gameplay authority
  until restoration completes; or feed the saved occurrence facts to session
  activation so the temporary population never exists. Owner:
  [`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).

✔ **D-SIM-SELECT — the last two selection sites now break ties on identity.**
The row named three; one was already closed (projectile-victim ties carry a
stable `(distance, x, y)` key). The two live ones were bare
`min_by(total_cmp)` on distance: possession candidates and the pickup magnet.
Both go through `sim_selection::winner_by(metric, SimId)` now — nearest first,
identity last — and both carry a spawn-order arm modelled on the projectile
tie-break's: the same two bodies spawned left-then-right and right-then-left,
with the identities fixed to POSITION rather than to the spawn slot, so "the
same winner" means the same body and not the same index. Each verified red by
poisoning its identity function to `None`, which is the exact shape of having no
tie-break at all.

✔ **D-REPLAY-RESIDUAL — the dead intent variants are gone and one listener was
measured redundant.** `LifecycleIntent` carried `DeathReset`, `ManualReset`,
`Replay` and `FullReset`; nothing recorded any of them and a stray one would
have returned `CommitOutcome::Retry` forever — a silent stall wearing an
exhaustive match's clothes. Deleted, with their codec branches; tags 0/1/2/4 now
refuse to decode. ⛔ THE READABLE SCHEMA DUMP COULD NOT SEE THAT: same stable
name, same encoder type, same projection, so every wire ledger stayed green
while the encoding changed. `GGRS_ROLLBACK_SCHEMA_VERSION` is what that class of
change is for — 139 → 140. Mary-O's `reset_snakes_on_room_reset` was deleted,
but only after a cover test was written: gutting it left all 41 Mary-O tests
green, which is evidence that nothing covered it rather than evidence of
redundancy. Gravity's `reset_gravity_on_room_reset` STAYS — `BaseGravity` is a
session-scoped resource and no room rebuild touches a resource. Two legs remain
unpinned and their tests say so: `return_the_replay_subject_to_spawn`'s use of
the admitted subject, and `follow_the_active_room`'s room memory.

✔ **D-RESTORE-FACTS — measured, and the SHAPE is provisionally kept.** A save
load builds its first room with no occurrence continuity and the file's facts
then correct it. Measured: the correction lands on the same authoritative
population an in-session re-entry produces, both for an empty file and for a
file carrying a relocated occurrence (`canonical_reconstitution.rs` cases 7-8,
both red under a poisoned `outlook_for`). ⚠ That is EVENTUAL convergence: the
fixture is not a real startup load, and the interim population is unproven
rather than proven harmless — see D-RESTORE-INTERIM above.
`ResetToCheckpoint`'s contract, which claimed it was "not a save load", is
reconciled.

✔ **D-SIM-LOCAL — the level's departure memory was three Bevy `Local`s.** The
row named Mary-O's `follow_the_active_room` room memory; the localizer named a
different system. `cycle_level_on_flag_tally` accumulated its dwell timer on the
SIM clock in a `Local<f32>`, and a Bevy local does not rewind — so a goal-pole
slide desynced the demo's own sync-test host at frames 92-93 on exactly the two
checksummed types the threshold gates (`MaryOLevelState`,
`PendingLifecycleCommit`). All three memories (dwell, the room the level asked
for, the room the mode last saw) now ride `LevelDeparture` on the mode owner,
registered and value-probed beside `MaryOLevelState`. ⚠ Only the dwell half is
PINNED: putting a `Local` back in `follow_the_active_room` leaves the new test
green, because a confirmed transition rebases GGRS and no rewind crosses the
commit frame. The move is still right — a correctness that holds only because
another layer rebases moves when the rebase does — and the test says so rather
than implying a guard it does not have. The row's quest/room-visit clause was
NOT a defect: it is a documented, checksum-guarded tradeoff
(`ambition_persistence/src/quest/registry.rs`).

✔ **D-POLICY-1 — `ambition_workspace_policy` is green, 35/35, with no bulk
waiver.** Twelve failures in five groups, each triaged to whether the ownership
rule had changed. Four policies described the SHAPE of the dead
`platformer_runtime` compat shim, whose own TODO asked for its deletion once
callers migrated — deleted, and replaced by the rule they stood in for
(`engine.no-generic-runtime-facade-in-the-actor-crate`). One pinned
`run_if(gameplay_allowed)` after the schedule replaced that repeated predicate
with the `GameplayGated` SET — a red for a change that made the gating
stronger. Three runtime dependencies the runtime already schedules
(`ambition_damage`, `ambition_mount`, `ambition_gameplay_trace`) were missing
from its allowlist. Four pose/velocity write sites were the ADR 0024 authority
forms the rule already sanctions elsewhere, in files never listed; one
(`officer_probe`) was a real bare write and now goes through `transit_body` via
a new `probe_stage::place`. `ambition_demo_smash/src/lib.rs` came under the size
limit by moving its 1640 lines of inline tests into sibling `tests.rs` files —
`module_size.toml` still has zero waivers. The velocity rule gained the poison
self-test its pose twin has always had: it had nineteen waivers and nothing
proving it still fired.

The one unresolved developer-policy choice from the session-ownership work is in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) §37.

## Current execution order

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
