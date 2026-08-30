# GPT review — the Rust correctness pass (relayed by Jon, 2026-08-29)

> ⚠ **This file is EVIDENCE, not status.** Its `▢ Open` list below is what was
> open ON 2026-08-29 and four of those rows have since landed. Whether a finding
> is still open lives in [the status ledger](review-findings-status.md).

Reviewed HEAD `23e472c39fd4`, source-level, **with no Cargo available in the
reviewing environment**. 24 findings: 8 P0, 12 P1, 4 P2. Its thesis is that a
focused repair phase should come before another monolith carve, because several
findings are *duplicate ownership systems actively generating bugs* rather than
merely slow compiles.

⭐⭐ **THE THESIS SURVIVED CONTACT.** Every P0 checked so far was real, and the
two sharpest were confirmed by MEASUREMENT rather than by reading:

| finding | how it was settled |
|---|---|
| submerged movement integrates twice | **10.8px against an authored 5.4 at 60Hz — ratio exactly 2.00** |
| sentry bolts cannot damage anything | the new end-to-end arm went red on the shipped code, twice, once per defect |

Status marks: ▢ open · ▣ done (with sha) · ⊘ retired with a reason.

## ▣ Landed 2026-08-29

- ▣ **P0 — submerged movement advances position twice.** `1b0bd665a`.
  `integrate_submerged_clusters` wrote both `vel` and `pos`; the shared sweep
  then advanced by that same velocity. ⛔ **the speed was the smaller half**:
  `stays_over_its_surface` validates ONE prospective step — it is what makes the
  trapdoor a door — and the sweep added a second, unasked step, so a body
  approved for a supported step landed past the lip. The neighbouring ledge test
  could not see it: it asserts WHERE she stops, and she stops at the lip either
  way. The new arm asserts the DISTANCE, plus `pos` moved by exactly `vel * dt`,
  which states the single-authority contract rather than the number.
- ▣ **P0 — sentry shots cannot damage anything.** `9a89c1039`. The turret is the
  bolt's `ProjectileOwner` and carries no `ActorFaction`, so nothing stamped —
  and `indiscriminate` is `allegiance.is_none() && owner.is_none()`, FALSE for a
  named owner, so `can_hit` was false against every victim alive. ⛔⛔ **the
  existing test passed throughout**, because it asserts a bolt APPEARS. The
  deployer's side is frozen onto the turret at deploy (the turret is the physical
  and presentation source and deliberately outlives its deployer, so the side
  cannot be looked up at fire time), and deployment is now ONE seam so a fixture
  cannot grant a faction production never grants.
- ▣ **P0 — held-item multi-seat loops abort later players.** `e3bc0924e`. Four
  per-body exits inside `for … in driven.entities()` were `return`. Seat zero is
  idle on most ticks, so this was the common case. Two arms, ordered by `SimId`
  so seat "a" is visited first; poison-checked both ways.
- ▣ **P0/P1 — eight event-created components now rewind.** `219539ab3` +
  `74c06f9d5`. Schema v134 → **v135**. `PlayerMark`, `BombFuse`,
  `GravityGrenadeFuse`, `PuppySlugAlly`, `FallingChest`, `CommandedMove`,
  `FallingHazard` (+ `MapEntities` and a mapping pass, because it names an
  Entity and drops on whatever it points at), and `HeldProjectile`.
  ⭐ **PROBED, NOT PRESENCE-ONLY** for seven of the eight — each is a NUMBER that
  decides an outcome, and a presence probe satisfies the coverage oracle while
  seeing none of it.
- ▣ **P1 — puppy-slug identities used a non-rewinding `Local<u64>`.** `74c06f9d5`.
  Replaced by the summoner's own rollbacked `SimIdCounter`.

## ⭐⭐ The finding under the findings, and it is the one worth acting on

⛔⛔ **THE EIGHT MISSING REGISTRATIONS ARE NOT EIGHT OVERSIGHTS.** The boot census
can only see components that exist in the INITIAL world. Every one of these is
inserted LATER — by an ability firing, a chest knocked loose, an encounter effect
attaching — so **nothing structural was ever in a position to ask** whether they
rewind. Expanding a hand-kept census does not close that.

The review's two proposed rules are the right shape and are recorded here rather
than half-built:

1. **A construction API for authoritative simulation entities** that makes stable
   identity and rollback participation hard to omit — mint the deterministic id,
   install the rollback lifecycle, record `SpawnOrigin`, and fail a STRUCTURAL
   test when a bundle's authoritative component types carry no snapshot/derived
   declaration.
2. **A repository rule for `Local<T>` inside rollback simulation:** a `Local` may
   hold only scratch whose value cannot affect authoritative output across
   invocations. A cached `QueryState` is fine; a cleared scratch vector is fine.
   Previous-room state, dwell timers, spawn counters and gameplay edge latches
   are not. ⭐ That one rule names the quest, map-visitation, Mary-O transition
   and puppy-slug findings as instances of one defect.

## ▢ Open, in the review's own order

⚠ **As of 2026-08-30 this list is stale in BOTH directions** — four rows below
have landed, and three of the rest were named by no later carry-forward list at
all (gravity, the trapdoor hold, the quadratic recorder). It is kept verbatim
because what the review said on the day is the evidence; the live answer is in
[the status ledger](review-findings-status.md).

- ▢ **P1 — Wire/Submerged can preserve an old `initial_dash_timer`.** Exclusive
  modes clear `dash_timer` and not this one; normal movement is what consumes it,
  so it freezes during the mode and resumes after. Wants ONE
  `interrupt_maneuvers_for_mode_transition()` authority.
- ▢ **P0 — Sentry/Vortex are autonomous gameplay state outside rollback.** The
  side is fixed; the turret still has no stable `SimId`, no rollback lifecycle
  anchor and no registered `Sentry`/`VortexWell`. ⚠ this is finding-1 territory:
  do it through the spawn API rather than by hand.
- ▢ **P1 — Sentry/Vortex read RAW faction, violating possession's effective-
  allegiance rule.** A possessed enemy is a Vortex target; a sentry deployed by
  one sees that body at distance zero as its nearest enemy.
- ▢ **P0 — gravity's declared authority does not rewind**, and
  `collect_gravity_zones` stores zones in QUERY ORDER while `gravity_dir_at`
  takes the first overlap — so an authored zone overlapping a grenade well has no
  authored answer. Wants priority + stable identity + a canonical sort.
- ▢ **P0 — Mary-O rollback can overwrite restored historical state.**
  `follow_the_active_room`'s `Local<Option<String>>` treats a historic room as a
  new transition and resets rollbacked `FlagSequence`/`MaryOLevelState`.
- ▢ **P1 — quest and room-visit edge detection is knowingly non-rewinding.** The
  repo's own checksum tests carry these as positive controls; the repair is to
  move the edge state into rollback, not to keep divergence merely detectable.
- ▢ **P1 — bomb/grenade "thrown" is inferred from velocity and is wrong BOTH
  ways.** Pickup zeroes velocity but leaves a lit fuse attached (an armed bomb
  ticks in hand); a fresh bomb falling under gravity reads as thrown. Wants an
  explicit `Throw`/`Drop` custody transition — arming reacts to the transition,
  not to a number.
- ▢ **P1 — held ranged shots are attributed to slot zero** (`PrimaryPlayerOnly
  .single()`, and the code says so). ⇒ fold held shots into
  `ProjectileSpawnRequest` and DELETE the parallel projectile simulation. That
  single change also takes the second world-collision implementation, the second
  portal policy, and the second place anti-tunnelling must be fixed.
- ▢ **P1 — D199's anti-tunnelling ray is a CENTRE LINE against an AABB body**, and
  it checks solids only, so a fast shot can graze a wall its centre misses and
  can cross a one-way from above. ⇒ swept AABB / Minkowski, policy-aware.
  ⚠ this refines the row already recorded in `queue.md` under D199.
- ▢ **P1 — submerged collision repair contradicts submerged passability.**
  `resolve_axis_repair` does not receive `BodyMode` and can depenetrate against
  blocks the sweep deliberately ignored, particularly on zero-delta axes.
- ▢ **P1 — custom held-item abilities are still singleton-controlled.** Beam,
  meteor, volley, vortex, grapple, blink, dive, mark/recall and puppy-slug all
  consume `ControlledSubject` while generic pickup/throw/fire iterate
  `DrivenBodies`. Wants ONE per-driven-body dispatcher.
- ▢ **P2 — nearest-target logic still ties on query order** in four places
  (projectile victims, sentry, possession, pickup magnet). Wants one shared
  deterministic selector ending in stable `SimId`, never `Entity`.
- ▢ **P2 — the trapdoor's "any non-movement action" is not what the kernel
  implements.** `UntilPressedAgain` says any action press except
  movement/jump/dash and checks only Attack or Special. ⭐ the review's framing is
  the useful part: `SmashChargeSpec` is being used for something that is not
  charging, and the abstraction wants to be a **timeline hold**
  (`hold_at`/`maximum_duration`/`roots_actor`/`release_policy`) with a smash
  charge as one customer.
- ▢ **P2 — the rollbacked input recorder scales quadratically.**
  `InputStreamRecorder` owns a growing `Vec` and the whole resource is cloned
  into every snapshot, so frame N copies history 0..N. Keep the finalized
  recording outside rollback; rewind a cursor and the unconfirmed tail.

## ⊘ The one the reviewer dropped, and it should stay dropped

⊘ **The Performer's starting down-Special releasing its own `UntilPressedAgain`
hold.** The reviewer traced the acceptance path and withdrew it:
`ProposedVerb::Special::spend()` zeroes the special buffer, so the buffered
intent is cleared before it can become the "new press" that releases the hold.
⚠ **`trap_probe`'s comments still imply the failure mode**, and they are what led
the review there — they should be corrected rather than left to send the next
reader down the same path.
