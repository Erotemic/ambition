# HEAD status

This is the canonical summary of the repository at HEAD. It records only current
state and current work. Historical execution narratives belong under
[`docs/archive/reviews/`](../archive/reviews/).

A status claim is acceptable here only when it has one of these evidence forms:

1. an executable test or policy;
2. a source owner whose type or constant directly establishes the fact;
3. a mechanically computed inventory;
4. an unchecked acceptance criterion that says exactly what is still absent.

Do not mark a track complete from a prose completion report alone. Re-run its exit
checks against HEAD and update this page in the same commit.

## Active architecture and verification work

| Workstream | HEAD state | Evidence | Completion evidence still required |
|---|---|---|---|
| Encounter orchestration | **PARTIAL.** Shared participant, objective, timeline, and music vocabulary exists, but boss and wave encounters still have different lifecycle/component schemas. | `crates/ambition_encounter/src/{state,participants,objective,timeline}.rs`; `crates/ambition_actors/src/boss_encounter/encounter_entity.rs` | One generic lifecycle/command ingress; objective-driven completion for non-boss cases; ownership-driven cleanup; snapshot-stable participant relations; a non-boss acceptance customer. |
| N3.2 exact restore | **OPEN.** Snapshot validation and several refusal paths exist, but cross-room rollback, unsupported dynamic reconstruction, and standalone preflight limitations remain explicit. | `crates/ambition_runtime/src/snapshot/mod.rs` (`RestoreError`, `validate_snapshot`, restore report) | Atomic room-context restore, reconstruction recipes, preflight before mutation, then a bounded rollback proof. |
| CC3 collision oracle | **DIAGNOSTIC, NOT A GATE.** The broad sweep is ignored and does not enforce completion thresholds. | `game/ambition_app/tests/collision_invariant_oracle.rs::collision_oracle_full_sweep` | A poison-tested policy for each illegal-state class and a non-ignored enforcement test. |
| BD5 boss validation | **DIAGNOSTIC, NOT AN INSTALL GATE.** The current fixture pins **8 errors / 10 warnings**. | `game/ambition_content/tests/boss_fight_validator.rs`; `validate_fight` has no content-install call site | Decide and implement the install-time rejection threshold; re-author content until the accepted threshold passes. |
| Sanic visible/playable recovery | **OPEN.** Windowed rendering and ball dash exist; the reusable selected-character and proven input path remain the acceptance target. | `game/ambition_demo_sanic_app`; `game/ambition_demo_sanic/src/ball_dash.rs`; [`demos/sanic-recovery.md`](demos/sanic-recovery.md) | Focused end-to-end input test, reusable selected-character binder, deterministic asset provisioning, and two-profile acceptance. |
| Super Mary-O | **PARTIAL.** Equipment data/mechanism, scroll policy, and flag sequence exist. Pickup wiring, live body-scale read-fold, and the full game shell remain. | `game/ambition_demo_smb1`; [`demos/super-mary-o.md`](demos/super-mary-o.md) | Pickup/equip path, collision/render scale consumption, enemies/HUD/results, and the headless 1-1 acceptance run. |
| Refactor R6e | **PARKED FOR A NAMING DECISION.** `player/` is gone; a half-rename of `features/` is explicitly rejected. | [`engine/refactor-chain.md`](engine/refactor-chain.md) | Jon chooses a full rename (`sim` plus coherent type names) or accepts the documented current name. |
| Large inline-test extraction | **INCOMPLETE.** Two production files still contain inline `#[cfg(test)]` modules of at least 200 lines. | Machine inventory in `scripts/check_agent_kb.py`; [`test-refactor-plan-2026-07-10.md`](test-refactor-plan-2026-07-10.md) | Extract both modules, or explicitly retire the 200-line threshold as a project rule. |

<!-- planning-evidence: boss-validator errors=8 warnings=10 -->
<!-- planning-evidence: inline-test-debt path=crates/ambition_characters/src/equipment.rs -->
<!-- planning-evidence: inline-test-debt path=game/ambition_demo_smb1/src/flag.rs -->

## Verified foundations

These are current facts, not active tasks:

- D-B's module-size policy counts physical source lines. It is green with one
  reasoned waiver for the declarative `game/ambition_app/src/menu/kaleidoscope_app.rs`.
- D-C's mode-scoped rules seam exists and is consumed by the demo rules crates.
- The generic platformer presentation plugin closes OV1 for Sanic and SMB1.
- Sanic's ball dash is implemented in its rules crate without a Sanic-specific
  engine primitive.
- Actor-local boss phase state is named `ActorPhaseState`; it is not encounter
  lifecycle state.
- The workspace policy package owns architecture-boundary enforcement. The
  deleted `game/ambition_app/tests/architecture_boundaries.rs` is historical only.
- Sprite-sheet embedding is owned by `crates/ambition_sprite_sheet/build.rs`.

## How to verify this page

Run:

```bash
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
```

Then run the Rust tests named by the owning workstream before changing a status.
A green documentation check proves the documented inventories agree with the
source shape; it does not substitute for the Rust behavioral suites.
