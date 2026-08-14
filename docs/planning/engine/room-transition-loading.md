# Room-transition loading — remaining work

> **Verified against `cecd01ca` (2026-08-13).** The readiness transaction,
> `RoomTransitionLoadState`, prepared construction, asset/readiness contribution,
> commit authorization, failure state, and neighbor-prefetch substrate are
> implemented. The old phase-by-phase campaign is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/room-transition-loading.md`](../../archive/planning-superseded/2026-08-13/engine/room-transition-loading.md).

## Remaining

- ▢ **Keep rollback-host transitions on the same prepare/readiness/commit
  transaction.** Reconcile confirmed-frame commitment with the normal transition
  transaction rather than maintaining two loaders.

  **Anchor recomputed 2026-08-14 (twice) and it still holds**, unchanged from the
  2026-08-09 census: fixed-tick host 11 room changes / 11 transactions / 0
  deferred; ROLLBACK host 24 / **0** / 24, with the shipped app host reporting
  `ConfirmedFrameBoundary present=true`. So the shipped game takes the
  transaction-free route on every room change.

  ⛔⛔ **BUT THIS ROW USED TO SAY "no rollback-host shortcut may … bypass the
  canonical construction plan", AND THAT DESCRIBES A BYPASS THAT DOES NOT
  EXIST.** Read `commit_transition` in `lifecycle_commit.rs`: it calls
  `RoomConstructionPlan::prepare`, preflights the subject, and then
  `plan.apply_to_world(world, carry_body)`. The confirmed path uses the SAME
  canonical construction the fixed-tick path does, and it even reuses
  `validated_spawn`. A transient `prepare` failure returns `Retry`, so it also
  already defers until the target is preparable.

  ⇒ **the gap is the READINESS TRANSACTION, not the construction.** What the
  confirmed route genuinely never runs is `RoomTransitionLoadState` and its four
  chained systems: the asset-readiness authorization, the presentation
  cover/`RoomTransitionPresentationAvailable` half, the unpresented-failure
  state, and `prefetch_hit` accounting. Someone acting on the old sentence would
  go looking for a second constructor to delete and find the one constructor
  already shared — and the real difference, that the shipped game changes rooms
  with no cover and no failure reporting, would survive the fix.

  ⚠ **size it before moving anything**, and it is small: `RoomTransitionSet` has
  exactly three members — `Detect` (1 system), `Apply` (the 5-system chain), and
  `Reset` (`reset_ecs_room_features`), plus `ContentRoomResetSet` anchored after
  `Reset`. All are registered in `app.sim_schedule()`, the rewound one.

  ⛔⛔ **AND THE CONSEQUENCE IS PLAYER-VISIBLE, not architectural tidiness.**
  `RoomConstructionPlan::prepare` requires only in-memory services — `RoomSet`,
  the catalogs, the lowering registry — and asks NOTHING about assets. So on the
  shipped host the target room is constructed the instant the frame confirms,
  while the destination theme's parallax layers lazy-load afterwards
  (`game_assets`: *"other themes lazy-load on room transition"*). The opaque
  cover that exists to hide exactly that gap is driven by
  `drive_room_transition_presentation` off `RoomTransitionLoadState`, which the
  confirmed route never populates — so **every room change in the shipped game is
  uncovered**, and the unpresented-failure state has nothing to report through.

  ⇒ **the concrete shape, which avoids the message trap rather than working
  around it.** The confirmed side already carries the intent in
  `PendingLifecycleCommit` — rollback-registered, read at the boundary — so it
  needs no message at all:

  ```text
  detection            (sim schedule)   records the intent, as it already does
  readiness chain      (confirmed side) driven from PendingLifecycleCommit,
                                        not from RoomTransitionRequested
  commit_confirmed_    (confirmed side) commits only once the transaction says
    lifecycle                           ready — the cover it already lacks
  ```

  The five Apply systems are not rollback-registered, which is what makes
  scheduling them beside the confirmed commit sound. The fixed-tick host keeps
  the message-driven path it already has; ⛔ do NOT make the confirmed commit emit
  `RoomTransitionRequested` — that message is `clear_message_on_rollback` and a
  rewind wipes it, which is the trap this direction exists to avoid.

  ⭐ **the two routes live in different schedules, and that is the actual
  obstacle.** The transaction chain (`begin` → `authorize` → `finalize` →
  `commit`) is registered in `app.sim_schedule()` — the REWOUND one under a
  rollback host — while `commit_confirmed_lifecycle` runs after `RunGgrsSystems`,
  deliberately outside it, because the load machine is not rollback-registered.

  ⛔ **so the obvious fix is unsound, and the repo already knows why.** Having the
  confirmed commit emit `RoomTransitionRequested` would put a message written
  outside the rewound schedule in front of a consumer inside it — and that
  message is registered with `clear_message_on_rollback`, so any rewind wipes it.
  That is the cross-frame-message trap, not a scheduling detail.

  ⇒ **the direction is to move the LOADER to the confirmed side, not to teach the
  confirmed side to load.** The transaction machinery is already not
  rollback-registered; scheduling it where the confirmed commit already runs is
  what makes one loader possible. Size that against the other `RoomTransitionSet`
  consumers before moving it.

- ▢ **Finish canonical plan/provenance convergence.** Transition, reset, and
  reconstruction should consume the same prepared construction semantics; remove
  any remaining family-specific reconstruction/legacy adapter only after its
  authoritative state/provenance is represented in the plan.

- ▢ **Close preload/performance behavior with measurements.** Exercise neighbor
  prefetch and promotion on real representative rooms, measure cover/transition
  latency, and improve the readiness pipeline where the data shows material
  stalls. Do not hide an unready feature by extending a cover indefinitely.

  ⛔ **do this AFTER the readiness convergence above, not before.** `prefetch_hit`
  lives on `RoomTransitionLoadState`, and the census re-measured on 2026-08-14
  says the rollback host — the shipped composition — opens **zero** transactions
  per room change. So that state is never populated on the route players take.
  Measuring now would profile the fixed-tick host's path and report numbers for a
  readiness pipeline the game does not run.

- ✔ **Prove possessed-body carry end to end.** Done 2026-08-14.
  `a_possessed_body_is_carried_through_a_room_transition` possesses an actor,
  stands THAT body in an authored `Door`, holds interact, and asserts the room
  changed, the driver survived the crossing, and the body arrived — 2003 px into
  `vertical_shaft`, not the few pixels gravity supplies, which is why the
  distance assertion is 200 px rather than nonzero.

  ⭐ **the branch was structurally untested, and the reason is worth keeping.**
  `carry_body` resolves to *"the controlled subject, unless it is the home
  avatar"*: the home body is moved by its own presentation path, whose query is
  `PrimaryPlayerOnly`, so a possessed body is precisely the one `carry_body`
  carries. Possession tests drove a body around one room; transition tests moved
  the home avatar. The composition was only ever inferred.

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
