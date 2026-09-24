# Control authority and AI policy are two facts in one component

> ⚠ **`Brain::Player` NO LONGER EXISTS** (re-measured 2026-08-20). `Brain` is a
> ONE-VARIANT enum — `StateMachine(StateMachineCfg)` — and who drives a body is
> the separate typed component `DrivingParticipant(PlayerSlot)`. Every mention of
> `Brain::Player` below is HISTORY describing the pre-split tree. ⛔ do not grep
> for it expecting to find the mechanism.
> ⭐ **Re-counted 2026-09-02: TWO mentions left, both comments** (was 29 on
> 2026-08-20 — the rest were tidied as their files were touched). Zero live code
> either way, so the claim held; only the number moved.

**Owner: the engine.** Written 2026-08-20 from Jon's architectural review of
`4af278e77`, which named this as the next broad direction after the custody and
construction work. ⛔ **the review also REFUSED the obvious version of it**, and
that refusal is the first thing to read.


## ⭐⭐ PREREQUISITE B — THE AUTHORITY, NAMED (2026-09-06)

> ⚠ **`TemporaryControl` NO LONGER EXISTS** (schema v208). It was a per-tick
> projection of `ControlClaims`; readers ask the claims directly
> (`ControlClaims::holds(ControlClaimant::Possession)` is what `Player` meant).
> Likewise `ScriptedControl` is gone: presence of `ControlHolds` is the
> suppression fact. Mentions below describe the tree before v208.

A review asks: *"Who owns transfer of control/custody between bodies? It should not
be: mount owns one version; possession owns another; abilities happen to host the
shared transition state."*

⇒ **The STATE authority already exists and is correctly placed.**
`ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl` is a
closed enum in the floor crate, carried across rewind by `SimId`, with exactly one
variant per mechanism:

```text
Autonomous                      the body runs its own brain
Player   { controller: SimId }  claimed by abilities::traversal::possession
Mounted  { mount: SimId }       claimed by ambition_mount
```

⚠ **THE WORD IS `claimed`, AND IT USED TO BE `written by`.** Since the arbiter
landed (next section) neither crate assigns the enum: they file and drop a
`ControlClaimant`, and `project_control_claims` writes the variant. The
paragraphs below are the ORIGINAL statement of the defect — kept because it is
why the arbiter exists, not because it still describes the tree.

⛔⛔ **WHAT WAS UNOWNED WAS THE TRANSITION, AND THAT WAS THE WHOLE OF PREREQUISITE B — ✔ CLOSED, see the next section.**
Two crates each `insert` their own variant directly, and **both release paths clear
to `Autonomous` unconditionally** — `possession.rs` on ending a possession,
`ambition_mount` on dismount — with no arbiter and no check that the other
mechanism has let go.

| failure, as it stood | mechanism | ✔ what closed it |
|---|---|---|
| a body both mounted and possessed | last writer wins; the other claim is silently discarded | two named claim fields, both live at once |
| releasing either one | sets `Autonomous` while the other claim is still live, so the body reverts to its own brain mid-ride or mid-possession | `drop_claim` clears ONE field; the projection re-reads what is left. Held across a rewind by `a_mount_dying_under_a_possession_survives_rewinds` |

⭐⭐ **SO IT IS A PRIORITY-CLAIM PROBLEM WEARING AN ENUM — and this repo already
has the pattern.** The music owner (`BOSS_MUSIC_OWNER` / `SCRIPT_MUSIC_OWNER`)
claims with an owner and releases only what it still holds; a peer found the
*identical* defect in the script half **on the same day** — a claim with no arbiter
and a release arm that does not ask whether somebody else still wants it.

## ✔ LANDED 2026-09-06 — the claim arbiter

⭐⭐ **`ControlClaims` in `shared_tangle::temporary_control`.** Both domains file a
claim instead of assigning the enum; `project_control_claims` decides the winner,
ordered by set after `PlayerSimulationSet::Possession` and `CombatSet::Settle` so
every claim filed this tick is counted.

| the review asked for | how it is answered |
|---|---|
| semantic owner identity, never `Entity` | `ControlClaimant` is a role; the subject is a `SimId` |
| deterministic precedence | the ENUM'S ORDER, in one place, nowhere else |
| multiple simultaneous claims | two named `Option<SimId>` fields, both live at once |
| release reveals the remaining winner | `release` clears ONE field; the projection re-reads what is left |
| rollback coverage | `actor.control_claims`, own codec, schema v164 → v165 |

⚠ **NAMED FIELDS RATHER THAN A COLLECTION, and the reason is not laziness.** Two
claimants exist, the precedence is total and known, and a fixed struct is
trivially clonable with no allocation and no ordering ambiguity — which matters
because a non-deterministic control mode is a rollback desync. A third claimant is
a field and a match arm, both of which the compiler makes you visit.

⛔⛔ **THE FIXTURE FOUND A SECOND DEFECT THE FIRST FIX WOULD HAVE HIDDEN.** The
production poison failed at SETUP, not at its assertion: the possession claim was
present and the ride claim was not. `ambition_mount` filed its claim only on the
tick the pair was ARMED, and `pirate_sky_lookout` authors a rider that is
*already* mounted — so it held a live ride with no claim to show for it, and the
steady-state arm was literally `(true, true) => {}`. ⇒ **The claim is the RIDE,
not the moment the ride began.** Reconciled in the steady-state arm now. A
constructed fixture would have boarded the rider and never shown this.

⛔⛔ **TWO DEFECTS IN THE FIRST LANDING, BOTH FOUND BY REVIEW, BOTH REPAIRED
2026-09-06.** They are worth stating because one of them was me widening a rule
whose text was three lines above the function I was editing.

**1 — the ordinary dismount never released the claim.** `apply_dismount_requests`
removes `RidingOn`, and the only other release arm queries bodies that HAVE
`RidingOn`, so after an ordinary dismount the rider could never reconcile: a
stale, rollback-canonical claim projecting `Mounted` for the rest of the match.
⇒ **A claim must be released where its FACT ends, not only where the dramatic
version of its ending is handled.** The death arm was the ending somebody thought
of; a lease expiring is the one that ships.

**2 — the claim conflated custody with control, and that one was mine.** My
steady-state reconciler filed a Mount claim for every live ride, on the reasoning
that *"the claim is the RIDE, not the moment the ride began"*. `board()`'s own
doc had already ruled otherwise, in as many words: `TemporaryControl` records
*"which transient controller is MASKING the body's autonomous brain"*, and
boarding masks a brain only when there is a `MountedBrainCache` to swap in — a
seated fighter keeps driving itself. ⇒ **The claim is the BRAIN SWAP.** A carried
body and a controlled body are different facts, and calling both `Mounted` made
the architecture describe a masking that never happened.

⚠ **AND THE OBVIOUS GUARD FOR #1 COULD NOT FAIL.** `smash_ride.rs` runs the whole
production road, so an assertion there looked right — but the Admiral is a
`board()` customer with NO cache, so he never files this claim, and the assertion
passed with the release deleted. ⇒ The subject of that defect is the CACHED rider,
and the guard had to construct one (`an_ordinary_dismount_releases_the_mount_claim`).
What `smash_ride` guards instead is #2, asserted WHILE HE IS ABOARD, because after
the dismount the claim is gone either way and the assertion cannot discriminate.

⛔⛔ **AND NARROWING THE CLAIM EXPOSED SOMETHING LARGER: `MountedBrainCache` HAS
NO PRODUCTION CONSTRUCTOR.** Measured 2026-09-06 across the whole tree — the type
is DEFINED (`ambition_mount/src/lib.rs:204`), READ as an `Option` by
`enforce_mount_rider_link`, ROLLBACK-REGISTERED as `mount.brain_cache`, and
CONSTRUCTED only inside `features/ecs/mount_pair_tests.rs`.

⇒ **So no shipped body is ever mount-CONTROLLED.** The brain-swap arm
(`(true, false)`, gated on `if let Some(cache)`) never fires, which means
`TemporaryControl::Mounted` was unreachable in shipped play before any of this
work, and `ControlClaimant::Mount` is a claimant with no production writer today.

⚠ **THIS DOES NOT UNDO THE REPAIR, AND THE DISTINCTION IS THE POINT.** The
PROJECTION was unreachable; the ERASURE never was. The mount's death arm writes
over the control mode for any `Mounted` rider whose mount dies — no cache
required — so a possession being erased by a dying shark was always reachable,
and that is what the poison still demonstrates (`left: Autonomous`,
`right: Player { slot:0 }`).

⚠ **AND IT IS THE THIRD BUILT-BUT-UNUSED CAPABILITY THIS WEEK**, after the
`EncounterScript` music owner with no `SetMusic` customer and the Limit meter with
no roster spender. ⇒ **Grepping for USAGE cannot find these; only the DEFINITION
side can.** A capability that is defined, read, registered and never constructed
looks exactly like a working one from every call site.

⇒ **STILL OWED:** a production test that rewinds ACROSS each transition and
asserts the effective authority comes back the same. The codec round-trip is
covered by a unit test and the schema is registered, but "restores the same
effective authority" is a claim about the rollback machinery, not about the codec,
and only a rewind proves it.

---

⇒ **The named authority is a control-custody CLAIM, not a new component.** It owns
`claim(owner, mode)` and `release(owner)` over the existing enum; `possession` and
`ambition_mount` become consumers that never assign `TemporaryControl` themselves.
⛔ The owner must be a **semantic identity** — `SimId` is already in both variants
— never a Bevy `Entity`.

⛔⛔ **CORRECTION, 2026-09-06 — I WROTE TWO THINGS HERE THAT WERE FALSE, AND A
REVIEWER CHECKED WHAT I ASSERTED.** Both are struck through below with what the
tree actually says, because the wrong version of this page was the reason the
row read as *latent* and got sequenced behind C2.

~~AND NO CURRENT TEST CAN FAIL ON THIS~~ — the accurate statement is narrower and
much less comfortable: no current test DRIVES the conflicting transition, but the
fixture that would host one is already authored and already runs.
`game/ambition_app/tests/carried_item_crosses_rooms.rs::a_mount_you_are_riding_crosses_the_door_with_you`
queries the assembled host for an entity carrying `RidingOn`, possesses that
rider, and asserts `PossessionState.possessed == Some(rider)` before carrying the
pair through a door. ⇒ The two-mechanism poison is not blocked on new content; it
is three lines away from a fixture that ships.

✔ **AND THE ACCEPTANCE IT OWED IS PAID. RE-DERIVED 2026-09-17 BY READING THE
TREE, because everything above this line describes a defect that no longer
exists.** The row asked for a two-mechanism poison — mount a body, possess it,
release one, assert the other still holds — on the grounds that neither release
arm asked whether the other claim was live. Both halves have landed:

* **Neither crate assigns `TemporaryControl` any more.** `ambition_mount` files
  and drops `ControlClaimant::Mount` (`file_claim` / `drop_claim`), and
  `abilities/traversal/possession.rs` files and drops
  `ControlClaimant::Possession`. The effective authority is a PROJECTION over the
  claims, which is exactly the "control-custody CLAIM, not a new component"
  design stated at the top of this section.
* ⛔ The `Autonomous` write that made the conflict player-visible is gone, and
  the comment where it stood says why: *"A dead mount ends the RIDE's claim; it
  does not make the body autonomous, and saying so erased a live possession."*
* **The poison exists and it is the stronger version.**
  `a_mount_dying_under_a_possession_leaves_the_player_driving` and
  `a_mount_dying_under_a_possession_survives_rewinds`
  (`game/ambition_app/tests/carried_item_crosses_rooms.rs`) drive the authored
  `pirate_sky_lookout` rider, kill the mount under the possession, and assert
  `holds(Possession) && !holds(Mount)` — the second one ACROSS A REWIND, and it
  also asserts the projection agrees with the claims.

⛔⛤ **WHAT THE TABLE OF "THE THREE WRITERS" USED TO SAY IS DELETED RATHER THAN
STRUCK THROUGH, and the reason is worth one line:** it cited two `lib.rs`
coordinates for writes that no longer happen, and a line citation to a repaired
site is worse than none — `check_planning_line_citations.py` reported one of them
as AMBIGUOUS because the line it named had been punctuation even when it was
written. A page that describes a fixed bug in the present tense is the rot this
campaign is about.

## ⭐⭐ PREREQUISITE B, MEASURED 2026-09-06 — THE SIX QUESTIONS, ANSWERED FROM THE TREE

The frontier marks control / possession / custody **DESIGN NEEDED** and the
architecture program lists six questions that must have crisp answers before the
control/body crates exist. Measured rather than designed, because four of the six
already have one answer and the interesting finding is *where* it lives.

⚠ **PRODUCTION WRITERS ONLY** — inline `#[cfg(test)] mod` blocks stripped. Counting
them put a test fixture among the answers on the first pass, which is the same
error `measure_kernel_module_graph.py` shipped with.

| authority | crate / module | writers 2026-09-06 | writers 2026-09-17 |
|---|---|---|---|
| `DrivingParticipant` | `ambition_characters::control` | **1** — `actor_monolith::control::authority` | **1**, unchanged — the sole-writer claim still holds |
| `PossessionState` | `actor_monolith::abilities::traversal::possession` | 2 — `possession.rs`, `control/authority.rs` | **3** — plus `session/teardown.rs`, which defaults it at session end |
| `TemporaryControl` | `shared_tangle::temporary_control` | 2 — `ambition_mount`, `possession.rs` | **0** outside its own module — the claim arbiter is the only writer |
| `ControlledSubject` | `shared_tangle::markers` | 2 — `possession.rs`, ~~`ambition_abilities::test_support`~~ (gated, below) | 2 — `possession.rs`, `session/teardown.rs` |

⭐ **RE-DERIVED 2026-09-17, and the row that moved most is the one this section
is about.** `TemporaryControl` went from two crates writing it directly to none:
both are claimants now, and `project_control_claims` is the only writer. ⚠ The
two rows that gained a writer gained the SAME one — `session/teardown.rs`
defaults both resources at session end — which is a lifetime edge rather than a
second authority, and it is worth a row because a census of "who writes this"
cannot tell those apart on its own. Method: `git ls-files` over `crates/` and
`game/`, skipping `*tests.rs` files and brace-matched `#[cfg(test)]` blocks, then
reading each surviving site; `vortex.rs` and `sentry.rs` insert
`DrivingParticipant` inside test modules and are correctly out.

### The answers

* **Who owns `PossessionState`?** `actor_monolith::abilities::traversal::possession`
  — ⛔ filed as a **traversal ability**, not as control authority. It is the same
  shape as the `assets -> session` edge: a thing filed beside its first consumer.
* **Who decides which body a participant controls?** `control/authority.rs`, and
  the doc claim that it is the SOLE writer of `DrivingParticipant` **holds** —
  one production writer, verified.
* **Who owns the transition between bodies?** `possession.rs`, de facto: it is the
  only file that writes THREE of the four types. ⇒ That is the control-transition
  authority, and its module path does not say so.
* **Who owns body custody when mounted / carried / possessed?** ✔ **ONE
  ARBITER, as of the section above.** It was two crates writing `TemporaryControl`
  directly with no arbiter — *"two mechanisms, one type"*, which was the
  question's real content. Both are claimants now and the projection owns the
  type; re-measured 2026-09-17, nothing outside `temporary_control` writes it.
* **Actor simulation state vs controller state?** The split is already clean at the
  TYPE level: simulation state is on the body, controller identity is
  `DrivingParticipant`, policy is `Brain`, and `ActorControl` is a separate
  component *"precisely so a brain swap cannot disturb the frame"*.
* **What is traversal capability state?** Unanswered here, and `PossessionState`
  living under `abilities/traversal/` is why the question is confusing: possession
  is currently *classified* as traversal.

⇒ **The topology finding: three of the four types are floor- or
domain-owned already (`shared_tangle`, `ambition_characters`), and the writers are
concentrated in two monolith files.** The carve is therefore not "extract control"
— it is **name the transition authority and move it out of the ability tree**,
after which `possession.rs` and `control/authority.rs` are one domain with a
module path that says so.

### ✔ AND ONE THING THE CENSUS FIXED ON THE WAY

`ambition_abilities::test_support` — a module whose own first line says
*"Test-only fixtures"* — was declared unconditionally and shipped in every build.
It writes `ControlledSubject`, so **a census of "who decides which body a
participant controls" counted a fixture among the answers**; an ungated
test-support module is indistinguishable from production authority.

⭐ The feature already existed (`test-support = []`) and the monolith's
DEV-dependencies already asked for it. Only the `#[cfg]` was missing. Now
`#[cfg(any(test, feature = "test-support"))]`.

⛔⛔ **AND THE FEATURE ALONE WAS WRONG, IN A WAY ONLY ONE OF THREE BUILDS COULD
SEE.** Gating on the feature broke `cargo test -p ambition_abilities` — the crate's
own tests use `crate::test_support` — while `cargo check --workspace --tests`
stayed **green**, because workspace feature unification turns the feature on for
everybody. ⇒ *A workspace check is not evidence that a crate builds alone*, and
the single-crate build is the only one that sees a gate like this.


## ✔ LANDED 2026-08-20 — `Brain::Player(PlayerSlot)` is DELETED

`Brain` is now `StateMachine(StateMachineCfg)` and nothing else. Who drives a
body is `ambition_characters::brain::DrivingParticipant(PlayerSlot)`, authored at
the spawn/seat site and moved for a possession by exactly one system,
`control::project_driving_participant`. `PossessionState::restore_brain` <!-- cite-ok: names a deleted field --> and
`restore_scope` are gone with it — a driven body keeps its own policy for the
whole possession, so there is nothing to stash and nothing to put back.

⚠ **`DrivingParticipant` stopped being a DERIVE in the same change.** Its
declaration's justification was *"reprojected from `Brain::Player` and possession
every tick"*, and that upstream no longer exists: the seat lives in that component
and nowhere else, so it is REGISTERED (`actor.driving_participant`,
`rollback_component_clone`) and `derived.driving_participant` left the schema.
`GGRS_ROLLBACK_SCHEMA_VERSION` 56 → 58 under Jon's standing 2026-08-08 ruling —
no migration, no shim. See `awaiting-maintainer-decision.md` §21.

⭐ **`Brain` is a ONE-VARIANT enum today.** Collapsing it into a struct is a
separate decision and was deliberately NOT taken here.

⚠ **the measurement below is PRE-DELETION** and is kept as the record of what the
conflation cost.

## What was refused, and why it matters more than what is proposed

> `Brain::Capability(BrainId)` + registered executable dispatch — that removes
> closed enum edges by adding a service locator.

An erased id plus a registry looks like decoupling and is the opposite: it
converts a compile error into a runtime lookup, and every question you could ask
the compiler ("does every policy handle this?", "is this policy reachable?")
becomes a question you can only ask a running process. ⛔ **no `Any`, no
`TypeId`, no `BrainId`, no executable registry, no service locator.** The same
prohibition that shaped `capability_lanes::CapabilityLanes`, for the same reason.

## The measurement

Taken 2026-08-20 against HEAD, so a later session can tell what moved.

```text
Brain                       2 variants   Player(PlayerSlot) | StateMachine(StateMachineCfg)
StateMachineCfg            12 variants   StandStill Patrol Wanderer MeleeBrute Skirmisher
                                         Sniper ChargeCrash BossPattern Smash Fighter
                                         Aerial PlayerDemo
Brain::Player             194 sites      across 14 crates/games
Brain::StateMachine       107 sites
exhaustive matches on Brain 13
StateMachineCfg::Fighter   20 external references
StateMachineCfg::Smash     13 external references
brain/fighter + brain/smash  8,950 non-test lines, INSIDE `ambition_characters`
```

⛔⛔ **THE CENSUS ABOVE IS STALE IN ALL BUT ONE LINE — RE-MEASURED 2026-09-02.**
It is the block this plan calls "the number to look at", so it is corrected here
rather than left for a reader to act on. Most of what it counts has already
happened.

⚠ **The exception, checked separately at `f563aa973`: `StateMachineCfg` had
exactly 12 variants and the carve changed none of them** — `StandStill`,
`Patrol`, `Wanderer`, `MeleeBrute`, `Skirmisher`, `Sniper`, `ChargeCrash`,
`BossPattern`, `Smash`, `Fighter`, `Aerial`, `PlayerDemo` <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
(`crates/ambition_characters/src/brain/state_machine/mod.rs:26`). The carve moved
the brains' IMPLEMENTATIONS out; it did not change the policy vocabulary, which
is the correct outcome and worth stating rather than leaving inside a blanket
"every line".

⛤ **RE-MEASURED 2026-09-18: ELEVEN, and the one that went is the interesting
part.** `PlayerDemo` was deleted with the player-clone hotkey (`89d78a4a5`) — <!-- cite-ok: the player-clone road was deleted in `89d78a4a5`; this line RECORDS the name, it does not point at one -->
it was the clone's brain and had no other user — so the live list is
`StandStill`, `Patrol`, `Wanderer`, `MeleeBrute`, `Skirmisher`, `Sniper`,
`ChargeCrash`, `BossPattern`, `Smash`, `Fighter`, `Aerial`.
<!-- cite-ok: `PlayerDemo` is named above as the variant that was REMOVED; the sentence is the record of it -->
⇒ The paragraph's POINT survives and is strengthened: the carve did not change
the policy vocabulary, and the only thing that has changed it since was deleting
a feature outright, which is the one change that should. A reader who spot-checks that one line and finds it right has no
way to tell whether the rest of the correction is trustworthy.

```text
                                  planned    2026-09-02   2026-09-17
Brain::Player                     194 sites   0            0 — the variant is DELETED;
                                                           the 2 greps are comments saying so
Brain::StateMachine               107 sites   118          122
exhaustive matches on Brain            13     moot: `Brain` has ONE variant now
StateMachineCfg::Fighter          20 ext.     24           28   outside `ambition_characters`
StateMachineCfg::Smash            13 ext.      9            9   outside `ambition_characters`
brain/fighter + brain/smash    8,950 lines   2,258        2,255 non-test remain inside;
  (non-test, inside characters)              6,491        6,315 in `ambition_combat/src/brain`
```

⭐ **THE 2026-09-17 COLUMN IS THE POINT OF KEEPING THREE: the remainder is not
moving.** 2,258 → 2,255 in a fortnight is three lines, on a number the acceptance
box treats as the work. The two that DID move are consumer counts —
`Brain::StateMachine` 118 → 122 and `StateMachineCfg::Fighter` 24 → 28 — so the
external surface of the policy vocabulary is growing while the floor crate's
share of it holds still. ⚠ That is the shape the orphan-rule paragraph below
predicts, and it is the reason a re-measurement is worth more than another carve
attempt: the pin is structural, so effort spent on the 2,255 buys nothing until
the pin is answered.

Method, so the next column is comparable: `git ls-files` over
`crates/ambition_characters/src/brain/fighter` and `.../brain/smash.rs`, skipping
`*tests.rs` files and brace-matched `#[cfg(test)]` blocks; the `ext.` counts are
`grep -rn 'StateMachineCfg::<V>' crates/ game/` minus lines under
`crates/ambition_characters/`.

⭐ **THE CARVE LARGELY HAPPENED ON 2026-08-27 (D168)**, and the acceptance box
below never learned. Both module headers say so in their first line: *"THE SMASH
BRAIN'S DATA — and only its data"*, *"THE FIGHTER BRAIN'S SHAPE — and only its
shape"*. The decision tick, option scoring, shadow rollout, recovery probe,
reeling response, charge maths, scenario suite, content schema, mode/action/emit
stages, difficulty filter and arena harness are all
`ambition_platformer2d::combat::brain::*` now.

⛔ **AND WHAT REMAINS CANNOT LEAVE — for a stated structural reason, not for want
of effort.** `Brain`'s snapshot encoder is bound to `ambition_characters` by the
ORPHAN RULE, and `ambition_combat` depends on `ambition_characters`, so a type
the encoder reads can never move up. `BrainSnapshot` pins more on top: `attack_kit`
is a `Vec<AttackCandidate>` BY VALUE, which is why the whole option vocabulary
stayed while its scoring went. ⇒ The remaining 2,255 lines (2026-09-17; 2,258 a
fortnight earlier) are DATA the floor crate must own. Anyone reading the
acceptance box as ~8,950 lines of pending carve is reading a number from before
the carve.

⚠ `ambition_characters` is still a floor crate every composition links, so the
question the plan asks is still live — it is just much smaller than stated, and
its remainder needs an ANSWER TO THE ORPHAN-RULE PIN rather than another move.

### The pin, located exactly (2026-09-02) — and the question it leaves

Two independent things hold the remainder in the floor crate. Both were located
rather than inferred:

1. **`impl SnapshotCursor for Brain`** (`crates/ambition_characters/src/snapshot_impls.rs`). `SnapshotCursor`
   is declared in `ambition_platformer2d_core`, so it is FOREIGN here; `Brain` is
   local. The impl is therefore legal only in `ambition_characters` or in
   `ambition_platformer2d_core`, and everything the encoder reads is pinned with
   it. Moving `Brain` "up" is not available — up is where the dependents are.
2. **`BrainSnapshot.attack_kit: Vec<AttackCandidate>`**, by value
   (`crates/ambition_characters/src/brain/snapshot.rs:87`). ⭐ This pin is STABLE rather than accidental:
   `ambition_combat` *consumes* that vocabulary
   (`use ambition_characters::brain::attack_kit::…` across `evaluation`,
   `rollout`, `decision`, `moveset`), which is the correct dependency direction.
   Nothing is upside-down; the floor crate owns a vocabulary the layer above
   reads, which is what a floor crate is for.

**And the size the decision should be made against**: re-derived 2026-09-17, the
remainder is **2,255 of `ambition_characters`' 24,006 non-test lines — 9.4%**,
and it is DATA. The original concern ("a movement-only game links two
platform-fighter policies") is about a tenth of one crate in inert definitions,
not about 8,950 lines of fighter AI.

⚠ **THE SHARE ROSE WHILE THE REMAINDER DID NOT, AND THE TWO FIGURES DO NOT
SUBTRACT.** This line read *"2,258 of 28,234 — 8%"* on 2026-09-02; the numerator
moved by three lines and the denominator by four thousand, so the percentage
moved because the CRATE shrank around a remainder that is pinned. ⛔ The earlier
denominator's method is not recorded, so 28,234 → 24,006 is not a measured
change: today's method is `git ls-files crates/ambition_characters/src`, skipping
`*tests.rs` files and brace-matched `#[cfg(test)]` blocks, which reads 37,965
total / 10,341 in test files / 24,006 non-test. Re-derive with that rule rather
than differencing the two.

### ✔ DECIDED 2026-09-02: DO NOT SPLIT. The pin is intentional.

This is an ENGINEERING call, not a product one, so it is answered here rather
than filed for the maintainer. Three shapes were available. All three were
measured, and two are refused by rules this repository already holds.

**A — a split encoder in the floor crate dispatching to a domain-owned codec.**
⛔ REFUSED BY THIS DOCUMENT'S OWN PROHIBITION. For `ambition_characters` to call a
codec owned by `ambition_combat` it must reach UPWARD, which needs a runtime
registry — and the "What was refused" section above rules out exactly that: *"no
`Any`, no `TypeId`, no `BrainId`, no executable registry, no service locator"*,
because it converts a compile error into a runtime lookup. The shape is not
available and the reason predates this question.

**B — a dispatcher trait the ggrs crate implements per variant.**
⛔ BLOCKED BY THE SAME ORPHAN RULE, one crate further up. `SnapshotCursor` lives
in `ambition_platformer2d_core` and `Brain` in `ambition_characters`; both are
FOREIGN to `ambition_platformer2d_rollback_ggrs`, so it may not write that impl
either. A newtype wrapper is the usual escape and does not fit here: the
registrar's bound is `T: Component<Mutability = Mutable> + Clone + SnapshotCursor`
(`crates/ambition_platformer2d_runtime/src/rollback/registrar.rs:77`), so the
COMPONENT itself must implement the trait — a wrapper would have to become the
registered component, which is a far larger change than the one being bought.

**C — move `Brain` up with its encoder.** The only shape that satisfies the orphan
rule, and the measurement kills it. Inside `ambition_characters` and outside
`brain/`, exactly TWO sites name the type — its registration
(`crates/ambition_characters/src/rollback_registration.rs:43`, `"actor.brain"`) and the encoder itself — so the
floor crate's actor model does NOT hold it back. What does is a crate above:
`ambition_mount` stores `pub brain: ambition_characters::brain::Brain` BY VALUE
(`crates/ambition_mount/src/lib.rs:205`) and depends on `ambition_characters`
but NOT on `ambition_combat`. ⛔ Moving `Brain` into a combat crate makes a MOUNT
system link one — which is precisely acceptance criterion 4 (*"a movement-only
game's linked-crate count does not rise"*) failing. The carve would buy 8% of a
floor crate by breaking the goal the carve exists to serve.

⇒ **The pin stays, and it is not debt.** Cost of this decision in the terms the
repo uses: **no schema bump** (nothing moves), **no crate gains a dependency**,
**zero call sites touched**. The encoder is
`crates/ambition_characters/src/snapshot_impls.rs:416-555` (re-read 2026-09-17; it
was cited at `:350-489` and the file has grown above it) and discriminates
exactly three variants (`BossPattern`, `Fighter`, `Smash`); the registrar is
generic over the trait, so there are no per-type call sites to migrate even if
one wanted to.

⚠ **What would REOPEN this**, stated so the decision is falsifiable rather than
permanent: `ambition_mount` ceasing to hold a `Brain` by value (the one measured
blocker), or `SnapshotCursor` moving somewhere both `Brain` and a domain codec
can see. Neither is worth engineering for its own sake.

## Acceptance

- ✔ `PossessionState::restore_brain` <!-- cite-ok: names a DELETED field --> is DELETED, not merely unused — **verified
  against the code 2026-09-02**: the resource has four fields (`possessed`,
  `home`, `hold_timer`, `prev_down_interact`) and no brain state. The name
  survives only in comments explaining what changed, and in an unrelated Yarn
  command (`<<restore_brain>>` / `cmd_restore_brain`), which is a dialogue verb,
  not this field.
  ⚠ **The "belongs to a bump" half cannot be checked, by design, and this row
  should not imply it can.** `RollbackRegistrationDescriptor` records
  `name`/`owner`/`kind`/`type_name`/`detail` — all TYPE-level — so adding or
  removing a FIELD of a registered resource moves neither `deterministic_dump()`
  nor `schema_fingerprint()`, and `rollback_schema_baseline` stays green.
  `GGRS_ROLLBACK_SCHEMA_VERSION` is the MANUAL knob for exactly the changes the
  fingerprint cannot see; that is what it is for, not a hole in it. ⇒ "was it
  bumped?" is answerable only by reading history, and I did not establish it —
  `git log -S` on the file finds comment-cleanup and rename commits, and the
  removal predates them.
- ✔ no exhaustive match anywhere has an arm for "a human is driving" beside arms
  for wanderers — **verified against the code 2026-09-02**: `Brain` has ONE
  variant (`StateMachine`), and `CharacterBrainTemplate`'s nine (`StandStill`,
  `Wanderer`, `MeleeBrute`, `Skirmisher`, `Sniper`, `ChargeCrash`, `Smash`,
  `Aerial`, `Fighter`) contain no player/human arm.
⇒ **ALL FOUR ARE SETTLED NOW.** Two were already met and had been carrying `▢`;
the other two are settled in the box below — one WITHDRAWN by the DECIDED
section, one met BY withdrawing it. The substance of the third is in the
re-measured census above: the BEHAVIOUR of both brains left on 2026-08-27
(6,315 non-test lines in `ambition_combat` at 2026-09-17), and the 2,255 that
remain are data pinned in place by the orphan rule. `Smash` and `Fighter` are
still variants of `CharacterBrainTemplate` — a template name, not 8,950 lines.

- ⊘ ~~`brain/smash` and `brain/fighter` leave `ambition_characters`~~ —
  **WITHDRAWN, by the DECIDED section on this same page.** The behaviour left on
  2026-08-27; what remains is 2,255 non-test lines of DATA held in the floor
  crate by the ORPHAN RULE, and all three shapes that could move it are refused —
  two by rules this repository already holds, the third because it makes a MOUNT
  system link a combat crate, which is the next row failing. ⛔ A reader meeting
  this box first must not start the carve: the decision is above and it is an
  engineering call, not a deferral.
- ✔ a movement-only game's linked-crate count does not rise — **met by NOT doing
  the row above.** It is the criterion that killed shape C.

⚠ **BOTH ROWS CARRIED `▢` WHILE THE SECTION ABOVE THEM SAID DECIDED, and one of
them was refuted BY the other.** That is the page disagreeing with itself in the
direction that invents work: an open checkbox is an invitation.

## ✔ The first slice is SPENT — re-derived 2026-09-17

> Evidence-driven carve; do not redesign the brain stack at once.

The sequencing this section prescribed was: *"introduce `ControlAuthority`, make
possession use it, retire `restore_brain`. Nothing moves crates."* All three have
landed, and this section had been describing them as next steps.

* The seam is `ControlClaims` / `ControlClaimant` in
  `shared_tangle::temporary_control`, with `project_control_claims` as the
  arbiter — the ✔ LANDED section above is its receipt.
* Possession and the mount are both consumers: they file and drop claims and
  assign `TemporaryControl` nowhere.
* `PossessionState::restore_brain` <!-- cite-ok: names a DELETED field --> is gone; the Acceptance list records it.
* **`Brain::Player` is named TWICE in the whole workspace, both times in a
  comment explaining what replaced it** (`avatar/bundles.rs`,
  `characters/src/control.rs`). This section carried *"named 194 times in 14
  crates. That is the real size of the first slice"* — that measurement is
  spent, and a size estimate for finished work reads as a warning about work
  ahead.

⛔⛤ **AND THE NAME THIS SECTION CHOSE IS TAKEN BY SOMETHING ELSE, which is the
one thing a reader still needs from it.** `ControlAuthority` EXISTS — in
`ambition_match::prepared` — and it is a different fact: which driver a SEAT
binds (`LocalInput` vs `Brain`), decided at match preparation. A reader following
this page's original instruction would find that type and wire control custody
into seat binding. The custody arbiter is `ControlClaims`; the seat's driver is
`ControlAuthority`; they are not two names for one thing.

⇒ And nothing is left of this page's plan to start. The Acceptance box is four
settled rows; the open question the page still owns is the ORPHAN-RULE PIN — not
a carve, an answer to why the floor crate must hold 2,255 lines of policy data —
and the DECIDED section states the three shapes that cannot supply one.

## Authority-first decomposition constraint

The [controlled-body plan](controlled-character-actor-kernel.md) and packet A4
separate proposing control from accepting a driving relation and from executing
body behavior. The accepted relation is the fact downstream interaction/combat
may consume; it is not a new owner of every ability's eligibility or cost.
Possession's current cycle with control can remain inside one coherent package.

Before extracting either side, enumerate all relation writers and release/reset
paths, then cover competing claims, two seats, mounted input, possession release,
actor removal and rollback. A broad context object carrying both authorities
would retain their coupling under a different name. Co-locating control-mode
transitions can be simpler than a claim registry with one actual mode customer.

Brain planners and remote agents propose bounded semantic intentions. Only the
same accepted-control/action road used by human input can apply them. Never give
a planner construction privileges or direct live-actor mutation to avoid normal
acceptance or scheduling.
