# Room-transition loading — remaining work

> **Verified against `cecd01ca` (2026-08-13).** The readiness transaction,
> `RoomTransitionLoadState`, prepared construction, asset/readiness contribution,
> commit authorization, failure state, and neighbor-prefetch substrate are
> implemented. The old phase-by-phase campaign is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/room-transition-loading.md`](../../archive/planning-superseded/2026-08-13/engine/room-transition-loading.md).

## Remaining

- ▢ **Keep rollback-host transitions on the same prepare/readiness/commit
  transaction.** No rollback-host shortcut may directly mutate the target room or
  bypass the canonical construction plan. Reconcile confirmed-frame commitment
  with the normal transition transaction rather than maintaining two loaders.

- ▢ **Finish canonical plan/provenance convergence.** Transition, reset, and
  reconstruction should consume the same prepared construction semantics; remove
  any remaining family-specific reconstruction/legacy adapter only after its
  authoritative state/provenance is represented in the plan.

- ▢ **Close preload/performance behavior with measurements.** Exercise neighbor
  prefetch and promotion on real representative rooms, measure cover/transition
  latency, and improve the readiness pipeline where the data shows material
  stalls. Do not hide an unready feature by extending a cover indefinitely.

- ▢ **Prove possessed-body carry end to end.** The commit path resolves the
  controlled subject and has `carry_body` plumbing; add a real possession → room
  transition → arrival exercise so the behavior is not only inferred from unit
  pieces.

- ▢ **Exercise loading-zone entry through the real movement kernel.** The
  detector now consumes `SweepSample` and the focused regression models a body
  stopped at a boundary, but it still constructs the kernel output directly. Add
  one end-to-end movement-kernel → loading-zone transition case so a future
  change to sweep publication cannot silently reintroduce the bug.

- ▢ **External/P2P coordinated commit is trigger-based.** When real netplay is
  built, give the external rollback host a peer-coordinated barrier/rebase seam
  and prove corrected-input cancellation. Do not build Matchbox ceremony before
  that trigger.

## Exit

Every room change has one inspectable target plan, one readiness result, and one
commit boundary; rollback hosts delay commitment without reimplementing room
construction; preload performance is measured rather than inferred.
