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

- ✔ **Exercise loading-zone entry through the real movement kernel.** Done
  2026-08-14. `the_real_kernel_publishes_a_sample_that_crosses_the_zone_it_was_
  stopped_on` builds real floor/wall geometry, walks a body east under the real
  movement model until the solver stops it against a band coincident with the
  wall, and hands the sample the KERNEL published — not one the test wrote — to
  the real `RoomSet::transition_for_player`. The poison half asserts the
  post-collision velocity still cannot reach the band. Probed: removing the
  kernel's `clusters.sweep` slot makes it fail, so it catches lost publication
  rather than only wrong consumption.

  ⭐ **and building it surfaced a timing fact worth stating.** The crossing lives
  in the ARRIVAL tick's segment — from short of the band to against it. Every
  later tick is pinned, and its segment is a POINT. A first attempt asserted on
  the pinned tick and failed for that reason. So the detector sees the crossing
  only on the tick the kernel publishes it: this is a same-tick contract between
  sweep publication and transition detection, not a state a later reader can
  observe.

- ▢ **External/P2P coordinated commit is trigger-based.** When real netplay is
  built, give the external rollback host a peer-coordinated barrier/rebase seam
  and prove corrected-input cancellation. Do not build Matchbox ceremony before
  that trigger.

## Exit

Every room change has one inspectable target plan, one readiness result, and one
commit boundary; rollback hosts delay commitment without reimplementing room
construction; preload performance is measured rather than inferred.
