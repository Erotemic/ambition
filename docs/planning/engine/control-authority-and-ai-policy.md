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

⚠ **The exception, checked separately at `f563aa973`: `StateMachineCfg` still has
exactly 12 variants and the same twelve names** — `StandStill`, `Patrol`,
`Wanderer`, `MeleeBrute`, `Skirmisher`, `Sniper`, `ChargeCrash`, `BossPattern`,
`Smash`, `Fighter`, `Aerial`, `PlayerDemo`
(`crates/ambition_characters/src/brain/state_machine/mod.rs:26`). The carve moved
the brains' IMPLEMENTATIONS out; it did not change the policy vocabulary, which
is the correct outcome and worth stating rather than leaving inside a blanket
"every line". A reader who spot-checks that one line and finds it right has no
way to tell whether the rest of the correction is trustworthy.

```text
                                  planned    measured 2026-09-02
Brain::Player                     194 sites  0 — the variant is DELETED; the 2
                                             surviving greps are comments saying so
Brain::StateMachine               107 sites  118
exhaustive matches on Brain            13    moot: `Brain` has ONE variant now
StateMachineCfg::Fighter          20 ext.    24 outside `ambition_characters`
StateMachineCfg::Smash            13 ext.     9 outside `ambition_characters`
brain/fighter + brain/smash    8,950 lines   2,258 non-test remain inside;
  (non-test, inside characters)              6,491 are in `ambition_combat/src/brain`
```

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
stayed while its scoring went. ⇒ The remaining 2,258 lines are DATA the floor
crate must own. Anyone reading the acceptance box as ~8,950 lines of pending
carve is reading a number from before the carve.

⚠ `ambition_characters` is still a floor crate every composition links, so the
question the plan asks is still live — it is just much smaller than stated, and
its remainder needs an ANSWER TO THE ORPHAN-RULE PIN rather than another move.

### The pin, located exactly (2026-09-02) — and the question it leaves

Two independent things hold the remainder in the floor crate. Both were located
rather than inferred:

1. **`impl SnapshotCursor for Brain`** (`crates/ambition_characters/src/snapshot_impls.rs:350`). `SnapshotCursor`
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

**And the size the decision should be made against**: the remainder is **2,258 of
`ambition_characters`' 28,234 non-test lines — 8%**, and it is DATA. The original
concern ("a movement-only game links two platform-fighter policies") is now about
8% of one crate in inert definitions, not about 8,950 lines of fighter AI.

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
(`crates/ambition_platformer2d_runtime/src/rollback/registrar.rs:67`), so the
COMPONENT itself must implement the trait — a wrapper would have to become the
registered component, which is a far larger change than the one being bought.

**C — move `Brain` up with its encoder.** The only shape that satisfies the orphan
rule, and the measurement kills it. Inside `ambition_characters` and outside
`brain/`, exactly TWO sites name the type — its registration
(`crates/ambition_characters/src/rollback_registration.rs:32`, `"actor.brain"`) and the encoder itself — so the
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
`crates/ambition_characters/src/snapshot_impls.rs:350-489` and discriminates
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
⇒ **TWO OF THE FOUR WERE ALREADY MET and had been carrying `▢`.** And the third
is mostly met too — see the re-measured census above: the BEHAVIOUR of both
brains left on 2026-08-27 (6,491 non-test lines now in `ambition_combat`), and
the 2,258 that remain are data pinned in place by the orphan rule. `Smash` and
`Fighter` are still variants of `CharacterBrainTemplate`, which is what the box
below is really still about — a template name, not 8,950 lines.

- ▢ `brain/smash` and `brain/fighter` leave `ambition_characters`, and the
  measured line count of that crate falls by roughly the 8,950 above — ⛔ a carve
  that only re-exports them has moved nothing, and the debt ledger must not be
  laundered: the destination joins in the SAME commit;
- ▢ a movement-only game's linked-crate count does not rise, and preferably falls.

## ⛔ How to sequence it, because the review said so twice

> Evidence-driven carve; do not redesign the brain stack at once.

The first slice is the SEAM, not the migration: introduce `ControlAuthority`,
make possession use it, retire `restore_brain`. Nothing moves crates. Only then
is there evidence about what a domain-owned policy component costs, and the
Smash/Fighter move is priced by measurement rather than by intent — the way
gravity priced the construction federation and produced
`capability_lanes::CapabilityLanes` instead of a third hand-written lane.

⚠ **`Brain::Player` is named 194 times in 14 crates.** That is the real size of
the first slice and it is why it is its own slice.
