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

  ⭐ **and AGENTS.md decides which side of the fork dies**, which the wording here
  had left open: *"Never fold a richer path onto a simpler one to 'preserve' it;
  make the richer/general path universal and delete the rest."* The RICHER path
  is the readiness transaction — it has authorization, a cover, a failure state
  and prefetch accounting. The SIMPLER one is `commit_transition`'s own
  prepare/apply. So the convergence is not "teach both to cooperate": the
  transaction becomes the one route, and `commit_transition`'s direct
  `RoomConstructionPlan::prepare` + `apply_to_world` is DELETED, leaving it the
  boundary gate it should have been. That is the deletion this slice owes.

  ⭐⭐ **AND THE FORK IS AT THE DETECTION SITE, WHICH MOVES THE WHOLE SLICE.**
  Measured 2026-08-14 in `world/rooms/systems.rs`: one decision, recorded two
  ways, chosen by host —

  ```rust
  if let Some(boundary) = boundary {
      pending_lifecycle.record(boundary.current, LifecycleIntent::Transition { .. });
      return;                       // rollback host: a rollback-registered INTENT
  }
  transition_writer.write(RoomTransitionRequested::new(zone, zone_sfx));
  ```                               // fixed-tick host: a cross-schedule MESSAGE

  The two representations fork at BIRTH, and neither contains the other: the
  message carries the resolved zone (target index, arrival, activation), the
  intent carries a stable `SimId` subject and the room id as a string — because
  the commit happens far from the zone that named it.

  ⇒ **so the convergence is not "give the transaction a second input", which
  would be the fork again in a new place.** Record the intent on BOTH hosts; make
  the transaction its only consumer; let the eager host's confirmation be
  immediate and the rollback host's be the confirmed frame.

  ⛔⛔ **AND THIS ROW USED TO SAY `RoomTransitionRequested` "THEN HAS NO PRODUCER
  AND IS DELETED". THAT IS FALSE AT HEAD, and it was false when it was written.**
  Loading-zone detection is one of FOUR production writers. Censused 2026-08-14
  (`grep -rn "RoomTransitionRequested" crates/ game/`, then every `.write(` site
  read):

  | # | production writer | what it means | subject it names |
  |---|---|---|---|
  | 1 | `detect_room_transition_system` (`world/rooms/systems.rs:284`) | a body crossed a loading zone — the EAGER half of the fork above | none (the commit re-resolves) |
  | 2 | `restore_checkpoint_on_session_start` (`shrine.rs:219`) | the save's checkpoint names a room this session did not open; a synthetic `Door` zone routes there once per session | none |
  | 3 | Mary-O level completion (`ambition_demo_mary_o/src/lib.rs:2566`) | the flag was touched; a synthetic `Door` zone leaves for the next level, RE-EMITTED every tick on purpose (see its own ⛔ comment) | none |
  | 4 | `handle_room_transition_presentation_events` RetryRequested (`world_flow/room_transition_presentation.rs:557`) | the player retried a FAILED load; it re-writes `active.request.clone()` | none — it inherits #1..#3's silence |

  ⇒ **the deletion is still owed, but it is GATED on all four, not on one.** A
  slice that migrates detection and deletes the message breaks checkpoint resume,
  Mary-O's flag, and Retry. ⛔ do not delete the type until a production reference
  census — not a test census; tests are not authorities — shows no remaining
  semantic role.

  ⭐⭐ **and the census hands the slice a bigger prize than one loader: the
  message CANNOT NAME ITS SUBJECT, and the intent already can.**
  `RoomTransitionRequested` carries `{ transition: { zone, target_room, arrival },
  zone_sfx }` and nothing about WHO is transiting, so
  `commit_ready_room_transition_system` re-resolves it at COMMIT time —
  `ControlledSubject` or a `PrimaryPlayerOnly` fallback — with a comment claiming
  *"this is the same subject the detect side resolves"*. It is a citation, and
  citations go stale: possession, death, or a control handoff between the request
  and the authorized commit silently transits a DIFFERENT body. `LifecycleIntent::
  Transition` records `subject: SimId` at DETECTION precisely so that cannot
  happen. AGENTS.md decides this the same way it decided the loader: the RICHER
  contract becomes universal.

  ⇒ **so the canonical description of a transition is semantic and
  subject-bearing** — `{ subject: SimId, target_room (stable id), arrival,
  edge_exit, zone_sfx }` — and the two hosts differ only in WHEN they hand it to
  the readiness transaction: the eager host immediately, the rollback host once
  its originating frame confirms. Two thin confirmation adapters, one definition
  of what a transition IS, one loader. ⛔ do not make the eager host pretend to
  have GGRS frame semantics to share code; the shared thing is the intent, not
  the waiting.

  **The field mapping, derived 2026-08-14 — the intent is a superset except in
  two places, and both are cheap:**

  | the transaction reads | the intent has |
  |---|---|
  | `transition.target_room` (an INDEX) | `target_room` as a room-id `String` ⇒ `RoomSet::room_index_by_id` |
  | `transition.arrival` | `arrival` |
  | `zone_sfx` | `zone_sfx` |
  | the crossing SUBJECT | `subject: SimId` — which the message does NOT carry, and is why the deferred path exists |
  | `transition.zone.id`, for `same_destination` dedup | ⚠ nothing |

  ⇒ the dedup term is the only genuine gap, and losing it is a FIX rather than a
  concession: `same_destination` ANDs in the zone id while its own comment says
  *"one transaction owns that destination; trigger noise is not a new request."*
  Two zones into one room currently open two transactions, which is the comment's
  opposite. ⛔ do that IN the convergence commit, where it has a reason; on its
  own it is a behaviour change with no symptom behind it.

  ⛔⛔ **BUT "KEY IT ON THE TARGET ROOM" IS THE OPPOSITE ERROR, and this row said
  it.** Two doors can lead to the SAME room at DIFFERENT arrivals — a room-only
  key collapses them into one transaction and the second crossing silently
  arrives at the first one's coordinates. The zone id is a proxy for the
  destination; the answer is not to keep the proxy or to drop to a weaker one,
  but to derive the key from the SEMANTIC intent: **`(subject, target_room,
  arrival, edge_exit)`**, still under the existing `session_scope` +
  `content_epoch` ownership. Trigger noise repeats all four and dedupes; two
  exits into one room differ in `arrival` and do not; and once a second
  participant can transit, two bodies differ in `subject` and do not either —
  which is why `subject` is in the key rather than merely in the payload.

  ⇒ **pin it with a poison.** A test that only proves repeated detection dedupes
  passes just as happily on a room-only key. The one that decides is *same room,
  different arrival, NOT deduped*.

  ⚠ the one fact to check first: `sim_schedule()` is a configured label, so the
  Apply chain moves by registering in `Update` rather than `app.sim_schedule()`.
  On a fixed-tick host whose sim IS `Update` that is a no-op; on one running
  `FixedUpdate` detection still precedes it within the frame; on the rollback
  host it leaves the rewind, which is the point.

  ⛔ **"move the Apply chain to `Update`" hides TWO responsibilities, and only one
  of them may move.** (A) HOST-SIDE READINESS — begin the transaction, prepare the
  target, gather asset readiness, check cover availability, authorize. None of it
  is rollback-registered and all of it belongs beside the confirmed commit. (B)
  AUTHORITATIVE ROOM RECONSTRUCTION — apply the prepared plan, transit the
  RECORDED subject, run the cross-domain room-change resets, emit the crossing's
  SFX/VFX, and under rollback rebase the session. That is simulation, and it must
  stay one operation.

  ⛔⛔ **the failure this prevents is TWO room mutations** —
  `commit_ready_room_transition_system` AND `commit_confirmed_lifecycle` both
  applying a plan. The rollback path has an atomicity property worth naming:
  it builds the replacement rollback session FIRST, mutates the world only then,
  and installs the rebase last. Preserve that ordering. The target is one
  *"apply this PREPARED transition to this RECORDED subject"* operation with two
  host wrappers that differ in bookkeeping (the rebase) and in nothing else.
  `commit_transition`'s own `prepare` + `apply_to_world` is deleted only once that
  canonical prepared commit can serve it — and ⛔ it is not replaced by a second
  exclusive `load_room` clone.

  ⛔ **scheduling is STATED, never inferred.** A set name configured in one
  schedule carries no ordering into another: re-using `RoomTransitionSet::Apply`
  in `Update` without configuring it there buys nothing. On a frame-stepped host
  simulation and readiness can both land in `Update`, so that host needs an
  explicit edge from transition publication to host-side readiness. ⛔ do not let
  Bevy's parameter-conflict resolution pick the order — that is luck with a
  deterministic-looking face on it.

  ⇒ **and if extracting the shared prepared-commit seam exposes an awkward
  crate dependency, that is EVIDENCE about the Bevy-crate boundary, not a problem
  to paper over with a callback or a context bag.** Record it.

  **Acceptance target, red today and ⚠ deliberately `#[ignore]`d**:
  `a_room_change_on_the_shipped_host_opens_a_readiness_transaction`
  (`game/ambition_app/tests/d71_transaction_census.rs`). Measured at HEAD: 60 room
  changes, 0 transactions, 60 deferred intents. It asserts what a player gets
  rather than which systems ran; when it passes, delete the `#[ignore]` rather
  than the test. ⛔ do not weaken or delete it because it becomes inconvenient.

  **What the slice owes beyond that one target** — behaviour, not system rosters:
  the eager loading-zone crossing enters the canonical transaction; the rollback
  one enters the SAME transaction only after confirmation; no target-room mutation
  happens before authorization; the cover/failure seam is reachable on the shipped
  host; **a possession or control change between request and commit does NOT
  change which body transits — the recorded `SimId` wins**; checkpoint resume,
  Mary-O level completion and Retry all travel the canonical path (Retry with the
  SAME subject and destination, not a freshly minted request that forgot both);
  repeated same-intent noise dedupes; **same room + different arrival does NOT**;
  the crossing SFX plays exactly once; a failed preparation leaves the SOURCE room
  authoritative; a recorded subject that has vanished CANCELS rather than
  substituting whoever is controlled now; and an externally-owned session still
  cannot rebase unilaterally.

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
