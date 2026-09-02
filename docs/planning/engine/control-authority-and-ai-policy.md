# Control authority and AI policy are two facts in one component

> ⚠ **`Brain::Player` NO LONGER EXISTS** (re-measured 2026-08-20). `Brain` is a
> ONE-VARIANT enum — `StateMachine(StateMachineCfg)` — and who drives a body is
> the separate typed component `DrivingParticipant(PlayerSlot)`. Every mention of
> `Brain::Player` below is HISTORY describing the pre-split tree; the repository
> has 29 such mentions left and all 29 are comments, zero live code. ⛔ do not
> grep for it expecting to find the mechanism.

**Owner: the engine.** Written 2026-08-20 from Jon's architectural review of
`4af278e77`, which named this as the next broad direction after the custody and
construction work. ⛔ **the review also REFUSED the obvious version of it**, and
that refusal is the first thing to read.

## ✔ LANDED 2026-08-20 — `Brain::Player(PlayerSlot)` is DELETED

`Brain` is now `StateMachine(StateMachineCfg)` and nothing else. Who drives a
body is `ambition_characters::brain::DrivingParticipant(PlayerSlot)`, authored at
the spawn/seat site and moved for a possession by exactly one system,
`control::project_driving_participant`. `PossessionState::restore_brain` and
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

⚠ **the 8,950 is the number to look at.** `ambition_characters` is a floor crate:
every composition links it, including the exploration game and the two fixture
consumers. Two platform-fighter policies live there because policy shares a
component — and therefore a crate — with control authority.

## The two facts

* **CONTROL AUTHORITY** — *who drives this body*. `Brain::Player(slot)` says the
  body reads participant `slot`'s control stream. This is participant-scoped and
  entirely generic: it names no policy, no genre and no content.
* **AI POLICY** — *how this body decides for itself*. `StateMachineCfg` is a
  closed set of typed policies, each carrying its own tuning AND its per-actor
  runtime state.

They are one enum, so they are mutually exclusive by construction. That is
convenient and it is why three separate costs exist:

1. ⛔⛔ **possession must SWAP the whole component and remember the old one.**
   `PossessionState::restore_brain: Option<Brain>` is rollback-registered state,
   so a rewound possession round-trips an entire AI policy's runtime state
   through a resource whose subject is *who is driving*. Possession should
   transfer control authority; it currently transfers a policy as well because
   it has no way to move one without the other.
2. ⛔ **"human participant" is an AI-backend variant.** A body under a
   participant's control is spelled as a case of the same enum whose other cases
   are wanderers and bosses. Every exhaustive match over `Brain` therefore has an
   arm for a thing that is not a policy at all.
3. ⛔ **policy cannot migrate to its domain.** A Smash fighter policy belongs with
   Smash; `Patrol` and `Wanderer` belong to the generic actor floor. They cannot
   move independently while both are variants of one enum in one crate.

## The shape

Two typed components instead of one enum, and NEITHER of them is erased:

```text
ControlAuthority   the participant slot this body reads      generic, engine-owned
AiPolicy           a typed, domain-owned policy component    domain-owned
```

* a player-driven body carries `ControlAuthority` and no `AiPolicy`;
* an autonomous body carries `AiPolicy` and no `ControlAuthority`;
* **possession INSERTS `ControlAuthority` and leaves `AiPolicy` where it is.**
  Release removes it. Nothing is stashed, so `restore_brain` retires and
  `PossessionState` stops carrying policy state through rollback.
* a domain publishes its own policy component. `Smash`/`Fighter` become the smash
  domain's; `Patrol`/`Wanderer`/`StandStill` stay on the actor floor. Each moves
  when it has somewhere to go, independently.

⭐ **this is the same federation shape as the checkpoint horizon and the
construction lanes**: composition names a domain's offer, never the concrete
types inside it, and the offer is a typed plugin rather than a table.

## Acceptance

- ✔ `PossessionState::restore_brain` is DELETED, not merely unused — **verified
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
⇒ **TWO OF THE FOUR WERE ALREADY MET and had been carrying `▢` — the two that
remain are the CARVE, and only the carve.** Checked 2026-09-02: `Smash` and
`Fighter` are still variants of `CharacterBrainTemplate` in
`ambition_characters`, so nothing below has moved.

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
