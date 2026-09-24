# Architecture review findings requiring implementation evidence

**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`, 2026-09-08.
This is a bounded source-review finding set, not a replacement issue tracker.
The [queue](../queue.md) chooses work and the
[frontier](actor-monolith-work-frontier.md) supplies packets. Remove a resolved
finding from the live work surface after recording its test and resulting owner.
Git preserves the previous diagnosis, including
[archived epochs](../repository-history.md). Locally absent history is not proof
of a fabricated source reference.

State on 2026-09-17, checked against the code: seven findings are closed (`F1`,
`F2`, `F4`, `F5`, `F7`, `F8`, `F9`), `F3` is half closed, and `F6` is an open
contract limitation. A closed finding is not a closed packet: `F8` closed while
`A12b` is open, and `F9` closed while `A1c/3-5` is open.

**Verification limit:** Rust/Cargo were unavailable in this review environment.
No Rust reproduction, gameplay session, rollback run, GPU run or release build
was executed. That limit applies to the review; the 2026-09-17 re-checks used a
working toolchain. The distinctions below are intentional:

- **Source-established:** the stated control/data flow is directly visible.
- **Conditional defect:** a supplied state can exercise a bad path; production
  reachability and the intended behavior still need the specified test.
- **Contract limitation:** an existing guarantee is narrower than a plan suggests;
  this is not automatically a new runtime regression.

## Closed findings

Git history has the baseline evidence for each closed finding.

- **F1 — startup checkpoint routing spent its latch before admission.** Fixed
  2026-09-08 by A1a/A1b in `session/checkpoint.rs`: the latch is written only
  when `Admission::admitted()`. Guard:
  `a_refused_slot_leaves_the_checkpoint_resume_retryable`.
- **F2 — boss damage paths read different geometry.** Closed by `A2a`:
  `apply_boss_hit` and the projectile preflight read published
  `DamageableVolumes`. A boss has no coarse fallback hull. Guards:
  `absent_unpublished_published_and_intangible_are_four_different_answers`,
  `a_boss_is_reached_only_through_its_published_volumes`. Do not make the
  renderer or texture quality authoritative for hurt geometry.
- **F4 — authored interaction values accepted without runtime semantics.**
  Resolved by deletion 2026-09-12 (`requires_facing`, `Pickup.collected`,
  `Chest.persistent`) and 2026-09-17 (`BreakableSpec.debris_cue`). The product
  choices stay open on Q63; see [item custody](item-custody-and-accounting.md).
  Use an unsupported-field diagnostic only for a field that the engine intends
  to support; delete a field that nothing plans to read.
- **F5 — the facade's render opt-out did not exclude the renderer.** Closed:
  the host's `ambition_render` dependency is optional, and the facade takes the
  host with `default-features = false`. `scripts/check_facade_dependency_closure.py`
  owns the closure count. The count is a lower bound: it excludes optional edges,
  feature activation, external packages and build/dev dependencies.
- **F7 — installation did not declare support.** Closed by `A11`:
  `TechniqueSupport` records one declaration per installing owner and refuses a
  second claim on a key; `MoveSpec::effect_refs` enumerates every effect site;
  `activate_staged_revision` refuses before publication. Guards:
  `the_barrier_closes_through_the_checked_road_under_the_real_lifecycle`,
  `the_shipped_composition_withheld_nothing_at_its_barrier`. Reject duplicate
  keys by default; never infer executable equivalence from metadata equality.
- **F8 — flow cursor width and non-finite timeouts.** Closed: `FlowNode` edges
  are `u16`, conversion is checked at the authoring boundary, and wait timeouts
  must be finite. The timeline still ends the move, so an unbounded wait only
  drops later `Emit`s. `A12b` still owes private fallible prepared constructors
  and a prepared revision pinned on the playback.
- **F9 — raw checkpoint reset readers mutated domains without admission.**
  Fixed 2026-09-08 by A1c/1-2: domain reducers read one
  `AdmittedCheckpointRestore` in an ordered `Admit, Apply, Retire` chain, and a
  refused request stays in `OutstandingCheckpointRequest`. Guards:
  `a_refused_reset_changes_no_domain_state_and_is_not_lost`,
  `domain_restoration_is_registered_in_the_commit_schedule_and_not_in_the_simulation`.
  The pinned-snapshot half is A1c/3-5; see the
  [checkpoint protocol](checkpoint-restoration-protocol.md).

## F3. ◐ HALF CLOSED — the obstruction bypass is fixed; swept target selection is not

**Owner:** projectile travel/contact selection. **Live status is owned by
[`projectile-contact-protocol.md`](projectile-contact-protocol.md), not by this
page** — checked 2026-09-17, and this heading routes there rather than carrying a
second copy that can drift out of step with it.

**A2b landed the half this finding leads with.** Both branches now sweep the
SHOT'S BOX under the shot's own `WorldHitPolicy`: the victim branch used to cast
`raycast_solids` at the victim's CENTRE with `include_one_way = false` hard-coded,
so a wall covering a body but not its centre did not block, a corner clip did not
block, and an `ExpireOnContact` shot damaged through a one-way its own contract
says ends it. The travel leg is captured once before integration, so the segment
cannot describe space the shot never crossed. Witness:
`a_one_way_blocks_the_shot_whose_policy_says_it_should_and_no_other`.

**What this finding asked for that is still open:** contact selection over the
traveled segment with a contact-time ordering — the protocol page's own words are
*"the swept-target half has not"* landed, and `A2c` is open there. ⇒ The
counterexamples below are still the specification for that half.

⚠ Below is the review's evidence at the baseline and is NOT a description of HEAD.

### Evidence, at baseline `300004d601af1e633cfaee969f079cf9bb368ca8`

In `crates/ambition_platformer2d_actor_monolith/src/projectile/systems.rs`, the
`UnresolvedFeatures` branch checks endpoint boss/breakable contact, emits a hit,
despawns and continues. World collision and its finite-body sweep occur after
that branch. Therefore the later sweep does not constrain every feature contact.
The ordinary body-victim path also selects by endpoint overlap, not a sweep over
the full body path, and its center-ray obstruction check is not identical to the
finite-size, projectile-policy-aware world sweep.

### Counterexamples to test

Use a straight, finite-size expiring shot with a thin blocking wall between its
previous location and an endpoint-overlapping breakable or boss. Advance the
actual production projectile step. Assert that the wall wins and the farther
feature is not hit. Put a normal actor wholly between start and endpoint to test
fast-body traversal. Add a corner clip where a center line is clear but the
projectile body is blocked. Test one-way policy separately for bouncing and
expiring shots.

**Do not claim these fixtures already pass or fail in this review.** The source
permits the bypass; a focused executable fixture must establish the reachable
case and keep any deliberate game policy explicit.

### Repair direction

A2b first fixes the obstruction bypass using the current travel policy and shape.
A2c then makes contact selection coherent across bodies, destructibles and world
geometry. Evaluate candidate contacts over the traveled segment(s), select by
contact time with a deterministic tie policy and apply the chosen travel outcome.
Do not sort endpoint centers and call it continuous collision detection. Preserve
returning shots, reflection, absorption, splash, already-hit lifetime and the
existing one-way policy. Do not combine all path/portal/moving-target generality
into an unreviewable first patch.

## F6. Construction verification detects invalid mutation; it cannot undo it

**Owner:** typed construction and lifecycle publication.
**Priority:** source limitation remains open; A10/I3b now has the explicit
repeated-development-reconstruction customer. I1/I2 do not depend on it.
**Confidence:** source-established contract limitation, explicitly acknowledged
by source comments; not a claim of a newly observed production corruption.

`crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs` gives
`ConstructionExecCtx` raw Commands. `ConstructionPlan` validates parameters,
stable IDs and relations before commit, then runs trusted recipe functions.
`verify_committed_roster` runs after mutation/flush and can reject a recipe that
removed a root, changed its SimId or created an undeclared occurrence. The source
explicitly notes that Bevy Commands do not roll back their mutations.

Consequently, failed preparation preserves the unmodified world, and failed
verification can prevent publication; failed verification does **not** by itself
restore the old world. A unit-returning recipe is not a proof of infallibility.
Trusted Rust can also mutate resources beyond an entity roster.

The immediate contract is fail-closed publication with a stopped/recovered host
on invalid trusted code, not continued simulation of a half-accepted revision.
For the supported reload path, build typed inactive candidate state, constrain
writes and prove its visibility/cleanup under A10 before claiming retained-scene
activation. Arbitrary native-plugin undo remains outside that contract. Arbitrary
Commands cannot provide that guarantee by documentation alone. Test an invalid
recipe that changes a root and one that writes an external resource; do not hide
the second case by checking only entity counts.

## A SECOND REVIEW, 2026-09-17 — six findings, and this page is not their home

⛔ **THIS PAGE'S NINE ARE A BOUNDED SET AT `300004d6`; these six are a different
review of a different window** (`90b7135..027915a`, 89 commits) and are recorded
here only so a reader asking *"what have reviews found"* gets one answer. Each
disposition was re-derived by opening the code before acting, and each landed
change carries its reasoning in its own commit.

| # | finding | disposition |
|---|---|---|
| 1 | Q142 framed three rollback defects as a maintainer decision | ✔ FIXED — `EncounterScript`, `ReleaseOnDeath` and `RecharacterizeBody` registered, schema v197 → v198, and **all three now have a poison-verified rewind witness** (2026-09-17); **TWO stay open, not one** — `PostBossNpc`, because its question is what an admitted REPLAY sweeps, and `SmirkingBehemothVictoryNpc`, which joined on 2026-09-18 when the instrument learned to see `game/ambition_content`. ⛔ This cell said "`PostBossNpc` stays open" until 2026-09-21; the owed LIST belongs to `scripts/check_presence_filtered_state_is_rollback_registered.py`, which names both and prints why each is not registered by analogy. Re-run 2026-09-21: **121 presence-filtered components across 21 registering crates, 92 registered, 27 waived, 2 owed** |
| 2 | the frame-zero carrier rebase cannot see a hidden construction candidate | ✔ FIXED, **twice — and the first fix refused the wrong thing** (see below). The INSTALLATION refuses now, before it mutates anything; ⛔ including candidates would be worse, because an order INDEX is positional and a candidate on one peer only shifts every index after it |
| 3 | the rebase fails open on a missing or duplicate `SimId` | ◐ FILED AT THE CODE — the alternative today is not a refusal, it is keeping the App-lifetime history, which is wrong by more. The condition that flips it is a real remote peer, and the comment says so |
| 4 | `clean_workspace_crates.sh` released the build lock before deleting | ✔ FIXED — an open descriptor is held across `du`/`find`/`mv`/`rm`. Measured both ways on a 9,000-file tree: 0 lock steals against 12. ⛔ **A second half of the same race was found by the follow-up review**: the `-e` test took NO lock when the profile had no `.cargo-lock` yet, so cargo could create and acquire it mid-delete. Apply mode now OPENS the path for append (creating it) and locks unconditionally |
| 5 | the Fade audit counted one authored fade where three ship | ✔ FIXED — `test_intro`, `intro_wake`, `drain_market_arrival`; 2.2 s of invisible wait across three rooms. The planning page that quoted the number was the second copy and is corrected too |
| 6 | the audit campaign is drifting into machinery `AGENTS.md` forbids | ◐ PART — the source-comment PATH gate is demoted to reporting; the writer-set ratchet keeps its ratchet and now states, at the top of the file, that moving a writer between schedules leaves it green and what behavioural arm should replace it |

✔ **ALL SIX RE-VERIFIED AGAINST HEAD, 2026-09-21, by opening the code and
running the instrument** — because four of them were still being carried forward
as open work by a standing goal, and a re-investigation that ends in "already
done" is cheap only the first time.

| # | what was checked, today | reading |
| --- | --- | --- |
| 1 | `check_presence_filtered_state_is_rollback_registered.py` | 121 / 92 registered / 27 waived / **2 owed**, and the two are `PostBossNpc` and `SmirkingBehemothVictoryNpc` — the row above is corrected to match |
| 2 | `lifecycle_commit.rs:190-200` | the post-commit refusal branch is GONE, with the comment saying it "can no longer be written"; the install is reached through the token |
| 2b | `local_session.rs:440-476` | a `── COMMIT: from here nothing may fail ──` marker sits AFTER both the build and `FrameZeroEligibility::check`, so every fallible preflight happens while the old session is still alive |
| 2c | `session.rs:652+` | the install re-censuses and `assert_eq!`s before its first destructive write — so "cannot fail" was the wrong paraphrase, corrected above |
| 4 | `awaiting-maintainer-decision.md` Q142 | already says the `RecharacterizeBody` arm landed, with its poison reading `[1, 1, 1, 1, 0]`; no contradiction remains on that page |

⚠ **THIS IS A RE-CHECK, NOT A NEW DISPOSITION.** Nothing moved from open to
closed here; what moved is that two sentences on THIS page had drifted from the
code they describe, which is the failure this page exists to avoid and had
started committing itself.

⛔⛤ **FINDING 2 WAS MARKED FIXED WHILE THE END-TO-END INVARIANT WAS STILL NOT
ENFORCED, AND THE FOLLOW-UP REVIEW OF 2026-09-17 CAUGHT IT IN ONE SENTENCE:
*"It refuses the rebase, not the session installation."*** The first repair put
the candidate check inside `rebase_rollback_carrier_order`.
`install_rebased_sync_test_session` then reset `RollbackFrameCount`, the
confirmation counter and the input authority, called the rebase, saw the refusal,
logged that the session *"starts on this App's earlier order history"*, reset
`GgrsTime`, and **installed the session anyway**. ⇒ It traded a later panic for a
new frame-zero timeline carrying every rollback order this App ever handed out —
which is the ID-PEER defect the rebase exists to remove, the one that produced 59
of 146 differing GGRS checksum parts between equivalent hosts.

✔ **CLOSED 2026-09-17 AT THE INSTALL ROAD — AND THEN CLOSED AGAIN, BECAUSE A
FALLIBLE INSTALL WAS THE WRONG SHAPE.** The first repair made the install check
first and return `Err(FrameZeroRefused)`. A third review pass found what that
cost: both callers then carried a branch for a refusal arriving AFTER their
destructive half — the room authoritative with the previous timeline's order
history installed, or no session at all — and *"continuing execution from there
is worse than terminating."* ⇒ The refusal moved to
`FrameZeroEligibility::check`, whose token is the only way to reach
`install_rebased_sync_test_session`, which has no RECOVERABLE failure. Both
impossible branches are DELETED rather than hardened into panics, because a
branch that does not typecheck needs no handler.

⛔ **THIS SENTENCE SAID "which cannot fail" UNTIL 2026-09-21, AND THAT IS NOT
WHAT THE FUNCTION SAYS ABOUT ITSELF.** It carries an unconditional
`assert_eq!` over a re-taken `census_rollback_carriers`, before its first
destructive write, precisely because the token proves the check RAN and not
that its answer still holds — so it can and does abort, it simply cannot hand a
refusal back to a caller that has nothing to do with one. "Cannot fail" invites
exactly the reasoning the 2026-09-17 review was written against. ⇒ The function's
own doc is the owner of this distinction and states it correctly; this page now
defers to it rather than paraphrasing it into something weaker. One count serves every reader
(`census_rollback_carriers`), because two spellings of "how many are hidden" is
how the two disagree.

⛔⛤ **AND THE SAME REVIEW FOUND THE REFUSAL HAD SIMPLY MOVED UP ONE CALLER.**
`maintain_local_session` stops the live session and then starts its replacement.
With the start fallible, a hidden candidate turned *"keep the old session and
retry"* into *"destroy the old session, fail to install its replacement, leave no
active session"* — introduced by the new failure mode in a caller nobody
changed. ⇒ The replacement road is PREPARE-then-COMMIT now: build the GGRS
session, take the frame-zero token, and only then stop and install. ⚠ The
`SessionSeatingSource::Pending` return has always had that shape and was moved
above the stop with it. `a_hidden_candidate_keeps_the_running_session_instead_of_replacing_it`
asserts the SESSION rather than the return value, because the session is what a
player loses; its premise arm proves the fixture can replace at all, so the
poison — putting the stop back ahead of the preflight — fails on *"leaving no
active session"* and not on the fixture.

⚠ **AND THE ARM IS ON THE INSTALL ROAD, WHICH THE FIRST ONE WAS NOT.**
`a_hidden_candidate_refuses_the_installation_and_mutates_nothing` builds a hidden
candidate, sets the frame counters to distinctive values (77 and 41, neither of
them the install road's 0 and −1), calls the installation, and asserts no session
exists, the counters are untouched, the order table is unchanged and no
`Time<GgrsTime>` was inserted. Poison-verified: restoring the old
install-anyway behaviour fails on *"a refused installation installed a session
anyway"* — the property, not the return value, because the `Err` is read last.

⚠ **WHAT IS NOT DONE:** the `CutsceneTriggerQueue` behavioural arm that would let
its ratchet be deleted. ⛔ It is now PRICED rather than pending: the arm the
ratchet's own docstring prescribed was built, and stayed green under both
poisons, because a room a world BOOTS into fires its binding before any rewind
window opens. The replacement needs a room TRANSITION taken mid-session. See
`a_room_cutscene_starts_under_a_rewind.rs`, which is kept for the weaker thing it
does pin and says so at the top.

## Investigation boundaries, not established bugs

1. Projectile leg reconstruction currently uses current position/velocity and dt.
   Acceleration, returning motion and portals may require actual path segments;
   inspect those implementations and construct a witness before asserting a bug.
2. Global room lookup and stable placement strings need a live-instance namespace
   for repeated simultaneous room instances. That is a multi-instance design gap,
   not proof that the current single-active-room profile is incorrect.
3. Prepared content metadata/fingerprint does not fingerprint arbitrary Rust
   function behavior. Same-build networking must identify compatible executables;
   do not infer a cross-build bug or demand a compatibility layer without checking
   the actual host handshake and the existing same-build policy.
4. Player-only stand breaking may be intentional. Generalizing all body filters
   while moving destructibles changes gameplay and requires a separately approved
   eligibility rule.

## Closure receipt

For each implemented finding, record the exact base, changed owner, reachable
fixture, test command/result, preserved behavior and remaining limitation. A
source inspection is not a passing runtime test; a unit fixture is not a GPU or
P2P receipt. Do not reopen the already repaired mark-clock, hit-flash, spawn or
portal-order defects merely because this review references their invariants.
