# HEAD status

Audited 2026-07-18 against the current source tree; amended 2026-07-19 by the
deep review ([`../archive/reviews/deep-review-2026-07-19.md`](../archive/reviews/deep-review-2026-07-19.md)).
This page records the live state and current work; completed execution
narratives belong in git history or `docs/archive/`.

## Closed architecture campaign

The July 15–16 architecture campaign is complete at its stated bar:

- activation, reset, transition, restore, and LDtk reload share one App-installed
  placement-lowering authority;
- `ambition_platformer_provider` owns the typed provider preparation/activation
  lifecycle;
- `SceneEntities` is gone and sequential session teardown/activation is covered
  through the real host lifecycle;
- `ambition_sim_harness` owns the reusable reset/step/action/observation surface;
- the named content families selected for eviction now register through open,
  content-owned seams;
- boss attack execution, timing, motion locks, and effects converge on
  `MovePlayback` and moveset data;
- domain plugins own the repaired dev/dialog/encounter/menu state families;
- touch semantics compile without the presentation stack; and
- render consumes the repaired combat/dialog read-model seams.

These are foundations to preserve, not active decomposition tracks.

## Current hard work

| Workstream | Current state | What closes it |
|---|---|---|
| Portal camera continuity | **RED on `main` (found 2026-07-21).** `app_it::portal_translation_camera_continuity` fails 2/3, so `./run_tests.sh` is 16/17 (the `workspace (default features)` job). Bisected to `294d7c85c` ("mint the frame-clock presented pose"): the camera now follows the PRESENTED body pose while the portal continuity pass measures its screen anchor against the AUTHORITATIVE post-transit body, so on the anchor frame the two are a whole portal separation apart and the camera stays at the entry. Not a test-only defect — the camera visibly lags through any portal transit whose pair is more than a few pixels apart, and every consumer mixing presented with authoritative positions has the same bug at any teleport. Full diagnosis, bisect table, instrumentation recipe, and the two attempted fixes in [`dev/journals/portal-camera-continuity-regression-2026-07-21.md`](../../dev/journals/portal-camera-continuity-regression-2026-07-21.md). | Make the presented pose current across a discontinuity (`presented_pose.rs` already has the `travelled_under_own_power` guard — find why it does not yield a current pose on the transit frame). That fixes the anchor frame and the release-frame pop together; patching either consumer alone only moves the failure. |
| GGRS correctness debt + effect quarantine | **OPEN (2026-07-19 deep review).** The registration set is comprehensive but has enumerated exceptions (unregistered sim-mutated state, cloned `live_boxes`, sim-scheduled `Local` state, two order-determinism holes), the deleted debt census left no coverage forcing function, and audio/VFX/persistence effects re-fire on resimulation (only `gameplay_trace` is quarantined correctly). A stale-base "Sprite work" commit also reverted two same-day FIX commits — re-fixed same day. | tracks #0 (debt) and #1 (quarantine); evidence in the deep-review doc §§1–3. |
| Encounter lifecycle convergence | **DONE (2026-07-16).** One command/lifecycle/objective authority (`EncounterLifecycle` + reducer + `EncounterCommand` ingress); ownership/policy-driven cleanup; `SimId::encounter` + snapshot-registered relations; consumers derive from lifecycle + staging policy; the Noether attunement is the shipped non-boss customer. E8–E13 all closed with exit tests in [`engine/encounter-orchestration.md`](engine/encounter-orchestration.md). | — closed; residual boss-owned pieces (outro-gated persistence, reward anchors, adaptive music) recorded there as actor-local/authored policy. |
| GGRS rollback integration | **DONE for the simulation harness (2026-07-18).** `ggrs`/`bevy_ggrs` now own frame history, save/load requests, rollback entity recreation, entity remapping, resimulation, and sync-test checksum comparison. The custom `ambition_runtime::snapshot` engine, restore transaction, coverage debt ledgers, and compatibility facade are deleted. The real `SandboxSim` can run under `SyncTestSession`, exact prepared-content/schema identity invalidates active sessions, and representative actor/projectile/encounter churn is exercised through real GGRS loads. ADR 0027 records the replacement. | Production online boundary: confirmed-frame quarantine for presentation/external effects, then a Matchbox-backed two-peer handshake through the same session seam. |
| Immutable prepared content and exact session identity | **DONE (2026-07-18).** Provider preparation validates and deterministically assembles one immutable `PreparedContent`; canonical roots own the exact object, fingerprint/schema identity, and App-local epoch. The identity now binds GGRS session startup rather than an Ambition-owned snapshot format. LDtk replacement is rejected while a rollback session is active and requires a coordinated restart. ADRs 0026–0027 record the contract. | Closed. Next world-construction milestone: explicit provenance plus one authored/staged/runtime-dynamic `ConstructionPlan` vertical slice. |
| Super Mary-O acceptance | **PARTIAL, engine seams proven.** Pickups/equip, grown form, ranged powerup, bricks, crony stomp behavior, flag sequence, clock, tally, cyclic restart, and (2026-07-21) a four-readout HUD through the new provider-declared HUD seam. | See the single-source remaining list in [`demos/super-mary-o.md`](demos/super-mary-o.md). |
| Sanic acceptance | **PARTIAL, movement/host seams proven; corrected 2026-07-19.** Persona, control chain, ball dash, transformation, lifecycle, route/momentum oracles, the ring economy, the badnik enemy loop, AND (2026-07-21) the on-screen ring tally through the new provider-declared HUD seam. | See the single-source remaining list in [`demos/sanic.md`](demos/sanic.md). |
| Fighter-brain L3 rollouts | **DESIGN CORRECTION REQUIRED.** The current proposal combines a wall-clock budget with deterministic authoritative simulation and proposes rollouts from a live snapshot despite the delayed `Perceived` contract. | Choose a deterministic work budget or recorded-input model, and define a rollout state built only from allowed perceived facts before implementation. |
| Boss animator residue | **BOUNDED.** The execution/body path is converged; remaining residue is animation vocabulary/projection (`BossAnim`→`CharacterAnim`, obsolete target mirrors where still live). | Complete the bounded animator fold. Do not reopen the already-shared body integration path. |

## Rollback terminology

`ggrs` is the rollback driver; `bevy_ggrs` is the Bevy world snapshot adapter.
Ambition has no independent ephemeral snapshot/restore engine. `SimId` remains
semantic identity, while `RollbackId` is GGRS frame-history identity.

The old atomic room-staging restore campaign remains useful history because it
discovered the authoritative state and construction boundaries, but its runtime
implementation has been removed. Ordinary activation, transition, reset, and
hot reload still use canonical construction. GGRS rewinds the ECS world directly.

## Deferred

- The final public name for the provider crate.
- A provider-owned placement-family channel beside the closed common Tier-0 schema.
- Menu-host extraction until a second real consumer exists.
- The boss-crate carve decision. Convergence permits reassessment, but no
  concrete dependency/build/reuse boundary is currently documented; the
  maintainer ruling remains open.
- A full `features/` rename; no partial rename.

Direct maintainer confidence belongs in
[`maintainer-decisions.md`](maintainer-decisions.md), not inferred from this
status summary.

## Mechanically recomputed evidence

These markers are cross-checked against live computation by
`scripts/check_agent_kb.py`; they exist so a claim in this file cannot quietly
drift from the tree. Regenerate by running that script and correcting the values
it reports. (Restored 2026-07-19: the 07-18 rewrite dropped them, which left the
KB check red.)

<!-- planning-evidence: boss-validator errors=8 warnings=10 -->
<!-- planning-evidence: workspace-members count=51 -->
<!-- planning-evidence: module-size waivers=0 unwaived-violations=0 stale-waivers=0 invalid-waivers=0 -->
<!-- planning-evidence: cc3 status=ignored -->
