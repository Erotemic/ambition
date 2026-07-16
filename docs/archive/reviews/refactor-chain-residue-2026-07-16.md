# Refactor chain — current residue

**Status:** R1–R5 and R6a–R6d are closed. R6e is parked for a naming
decision. Encounter convergence is tracked separately and remains partial.

The detailed execution record through 2026-07-11 is archived at
[`docs/archive/reviews/planning-history-2026-07-11/refactor-chain-through-2026-07-11.md`](../../archive/reviews/planning-history-2026-07-11/refactor-chain-through-2026-07-11.md).

## Durable conclusions

- No further `ambition_actors` crate split is owed by the decomposition plan.
  Compile-time evidence showed that another carve would not address the dominant
  relink cost.
- The remaining tree should improve by deleting adapter layers and duplicate
  authorities, not by adding facade crates.
- `player/` no longer exists as a sibling simulation path. Player-ness is a
  controller/slot role; the home avatar remains a real slot-scoped concept.
- The mode-scope seam, collision-world ownership move, `ControlFrame` allowlist,
  and actor/control vocabulary moves are established guardrails.
- The old encounter R7 completion grade is withdrawn. Shared vocabulary landed,
  but lifecycle, cleanup, command ingress, and snapshot convergence remain open
  in [`encounter-orchestration.md`](encounter-orchestration.md).

## Closed slices

| Slice | Current result | Evidence owner |
|---|---|---|
| R1 mode scope | Hosted rules can wake by authored room mode and mode-scoped entities are retired on mode exit. | `ambition_runtime::mode_scope`; demo rules tests |
| R2 GNU-ton teardown | Fused profile/split-layer render duplication was removed; the measured premise that the whole boss directory was an adapter shell was rejected. | boss profile/render tests |
| R3 collision overlay split | Collision world lives with world geometry; mechanic-specific geometry helpers remain below it. | `ambition_world::collision`; architecture policy |
| R4 projectile steppers | One owner moved; two candidates were intentionally stopped because required view/control seams were absent. | archived execution record |
| R5 `ControlFrame` lint | Global-frame consumers are allowlisted and poison-tested. | workspace policy determinism/control checks |
| R6a–R6d player fold | Body, control, affordance, mechanics, and avatar concepts were redistributed; `crates/ambition_actors/src/player/` is gone. | source tree and module map |

Closed slices should not be reopened from their historical prose. Reopen only
when a current failing invariant or a new customer demonstrates a concrete gap.

## R6e — `features/` naming decision

The remaining name `features/` does not mean Cargo features. It contains the
actor/prop simulation tree and related `Feature*` identifiers.

A prior estimate treated this as a directory-only rename. Source inventory showed
that a coherent rename also affects a broad public type family across multiple
crates. Renaming only the module would create a worse mismatch:

```text
crate::sim::FeatureId
crate::actors::FeatureSimEntity
```

That half-rename is forbidden.

### Option A — coherent rename

Rename the module and the type family together. The leading candidate is:

```text
module: sim
FeatureId -> SimEntityId
FeatureSimEntity -> SimEntity
FeatureView -> SimEntityView
...
```

Requirements:

- choose names that do not collide ambiguously with Bevy `Entity`;
- update all defining and consuming crates atomically;
- regenerate module maps;
- keep architecture and determinism policy green;
- do not add compatibility re-exports.

### Option B — accept the current name

Keep `features/` and the `Feature*` vocabulary, with the module map explicitly
explaining that the term means in-world simulation entities rather than Cargo
features.

This is acceptable if the churn of a coherent rename is not worth its benefit.

### Decision required

Jon chooses A or B. The executor must not choose a module-only compromise.

## Separate active work

These items are not part of R6e and must not be hidden inside the naming sweep:

- encounter lifecycle convergence;
- exact snapshot/restore work;
- Sanic selected-character/input composition;
- deferred avatar projectile-spawner unification, which changes feel and requires
  differential/interactive evidence.

## Refactor acceptance discipline

For a future refactor slice:

1. Measure the actual owner/consumer set first.
2. State units for every count.
3. Name the authority that will be deleted, not only the type that will be added.
4. Add or identify an invariant that fails if the old path survives.
5. Update [`../status.md`](../status.md) and [`../tracks.md`](../tracks.md) in the
   same commit.
6. Archive detailed execution notes after the slice settles.
