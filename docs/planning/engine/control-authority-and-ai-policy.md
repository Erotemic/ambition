# Control authority and AI policy are two facts in one component

**Scope:** the split between who drives a body (control authority) and what the
body's own brain would do (AI policy). **Owner:** the engine. The split is
built. Open work is in A4 of the [frontier](actor-monolith-work-frontier.md) and
in the owed rewind witness below.

## Current model

- **Who drives a body** is `ambition_characters::brain::DrivingParticipant(PlayerSlot)`,
  set at the spawn/seat site and moved for a possession by one system,
  `control::project_driving_participant`. It is rollback-registered
  (`actor.driving_participant`). `control/authority.rs` is its only production
  writer.
- **AI policy** is `Brain`, a one-variant enum (`StateMachine(StateMachineCfg)`).
  There is no player variant, so no exhaustive match mixes "a human is driving"
  with wanderer arms. A driven body keeps its own policy for the whole
  possession; nothing is stashed or restored. Collapsing `Brain` into a struct is
  a separate decision, not taken.
- **Temporary control is a claim.** `ControlClaims` in
  `shared_tangle::temporary_control` holds one named `Option<SimId>` field per
  `ControlClaimant` (`Possession`, `Mount`). Domains file and drop claims through
  `file_claim` and `drop_claim`. There is no stored winner and no per-tick
  projection: a reader asks `ControlClaims::holds(..)` for the claim it cares
  about, so no projection can lag the claims. Precedence is the enum's order
  (possession outranks a ride). The claims are rollback state
  (`actor.control_claims`).
- **Named fields, not a collection:** two claimants exist, precedence is total,
  and a fixed struct clones without allocation or ordering ambiguity. A third
  claimant is a field and a match arm that the compiler makes you visit.
- **Release reveals the remaining claim.** `drop_claim` clears one field; the
  other stays. A dead mount ends the ride's claim; it does not make the body
  autonomous.
- `ActorControl` is a separate component so a brain swap cannot disturb the
  frame.

Rules learned from the claim arbiter:

- Release a claim where its fact ends, not only where the dramatic version of its
  ending is handled (an ordinary dismount releases the mount claim:
  `an_ordinary_dismount_releases_the_mount_claim`).
- The mount claim is the brain swap, not the ride. A carried body and a
  controlled body are different facts; a seated fighter keeps driving itself.
- `MountedBrainCache` has no production constructor, so no shipped body is
  mount-controlled today and `ControlClaimant::Mount` has no production writer.
  Q76 (2026-10-03) keeps the capability: give it an early customer and restore
  the mount's previous brain on dismount ([queue](../queue.md#mount-rider-customer--a-shipped-rider-controls-a-shipped-mount)).
  The erasure of a possession by a dying mount was still reachable, and is
  guarded.

Witnesses: `a_mount_dying_under_a_possession_leaves_the_player_driving` and
`a_mount_dying_under_a_possession_survives_rewinds`
(`game/ambition_app/tests/carried_item_crosses_rooms.rs`), and the A4 acceptance
table in the frontier.

**Owed:** a production test that rewinds across each control transition and
asserts that the effective authority comes back the same. The codec round-trip is
unit-tested; only a rewind proves the restore.

### Name collision

`ControlAuthority` exists in `ambition_match::prepared` and is a different fact:
which driver a seat binds (`LocalInput` versus `Brain`), decided at match
preparation. The custody arbiter is `ControlClaims`. Do not wire control custody
into seat binding.

## Where the writers are

| authority | crate / module | production writers |
|---|---|---|
| `DrivingParticipant` | `ambition_characters::control` | `actor_monolith::control::authority` |
| `PossessionState` | `actor_monolith::control::possession` | `possession.rs`, `control/authority.rs`, `session/teardown.rs` (defaults it at session end) |
| `ControlClaims` | `shared_tangle::temporary_control` | `file_claim` and `drop_claim`, called by the possession and mount domains |
| `ControlledSubject` | `shared_tangle::markers` | `possession.rs`, `session/teardown.rs` |

`session/teardown.rs` is a lifetime edge, not a second authority. A census of
"who writes this" cannot tell those apart by itself.

The writers concentrate in the monolith's `control/` module:
`control/authority.rs` and `control/possession.rs` are one domain with a module
path that says so. Possession eligibility stays optional ability policy.

Test-support modules must be gated (`#[cfg(any(test, feature = "test-support"))]`);
an ungated fixture that writes control state reads as production authority in a
census. A workspace check does not prove a crate builds alone: feature
unification hides a missing gate, so also build the single crate.

## Refused: an executable brain registry

> `Brain::Capability(BrainId)` + registered executable dispatch — that removes
> closed enum edges by adding a service locator.

An erased id plus a registry converts a compile error into a runtime lookup.
Questions such as "does every policy handle this?" become questions only a
running process can answer. No `Any`, no `TypeId`, no `BrainId`, no executable
registry, no service locator. The same prohibition shaped
`capability_lanes::CapabilityLanes`.

## Brain data stays in `ambition_characters`

The fighter and smash brain behavior lives in `ambition_combat/src/brain`. What
remains in `ambition_characters` is data (about a tenth of the crate's non-test
lines): `StateMachineCfg`'s variants and the snapshot vocabulary. It stays, for
two located reasons:

1. `impl SnapshotCursor for Brain` (`crates/ambition_characters/src/snapshot_impls.rs`).
   `SnapshotCursor` is declared in `ambition_platformer2d_core`, so by the orphan
   rule the impl can live only in that crate or in `ambition_characters`, and
   everything the encoder reads is pinned with it.
2. `BrainSnapshot.attack_kit: Vec<AttackCandidate>` by value. `ambition_combat`
   consumes that vocabulary, which is the correct dependency direction.

Three ways to move it were considered and refused:

- **A split encoder dispatching to a domain codec:** needs a runtime registry,
  refused above.
- **A dispatcher trait implemented in the GGRS crate:** blocked by the same
  orphan rule; a newtype would have to become the registered component.
- **Move `Brain` up with its encoder:** `ambition_mount` stores a `Brain` by
  value and does not depend on `ambition_combat`, so a mount system would link a
  combat crate. That fails the acceptance "a movement-only game's linked-crate
  count does not rise".

The pin is not debt: no schema bump, no new dependency, no call sites touched.
Reopen only if `ambition_mount` stops holding a `Brain` by value, or if
`SnapshotCursor` moves somewhere both `Brain` and a domain codec can see.

## New form customer: presentation and ability grants are not primary-player-only

Mary-O's `cosmic::toggle_cosmic_form` (2026-10-11) selects the body from
`ControlledSubject`, so a possessed/selected body can become cosmic. The
same module's `lend_the_jump_in_the_air`, `shorten_her_hit_stop`, and
`play_cosmic_music` filter their queries with `With<PrimaryPlayer>`.
The selectors do not mean the same thing. A non-primary controlled body can
therefore wear the cosmic catalog row without receiving its jump grant,
hitstop rule or musical claim. This is a direct customer for the existing
relativity rule, not evidence for another global active-player component.

**Target:** if a mechanic depends on the character's current form, derive its
body-local grants and combat policy from each eligible body wearing that
form. An input press targets the actor assigned to that participant; the
rest of the form does not depend on which seat is designated primary. A
music claim is keyed to the form's source/body occurrence and its live room,
through the existing priority music protocol. If only Mary-O is eligible,
the admission predicate is its catalog/ability, not an implicit seat number.
Do not make `ControlledSubject` the global answer for every game's action.

**Witness:** give the same cosmic form to the primary body and to a second
eligible, driven body; both get the jump, hitstop and visual semantics.
Then change a body out of the form, despawn it or move it to another live
room. Its contributions and room music claim retract without affecting the
other body's state. Verify the current single-player Mary-O behavior and
rollback. Poison: restore `With<PrimaryPlayer>` on a body-local form grant;
the second-body fixture must fail.

### Scripted movement cannot borrow combat recoil as a control lease

**What landed (2026-10-12):** the pipe ride holds
`ControlHold::Transit` (the existing `ControlHolds` gate, which blanks the
frame at `blank_scripted_control_frames`) and `PoseHold::Transit`. It
claims both on entry and on each tick of the ride, and releases only those
two at the end. `TRANSIT_LOCK` and its write to `recoil_lock_timer` are
deleted. Witness: `a_pipe_ride_leaves_a_recoil_and_another_holders_hold`
(a recoil set mid-ride outlives the ride; another pose holder keeps its
hold). Not witnessed: early cancellation, rollback restore, and two bodies
in pipes at once. The text below is the case as found.

**New immediate customer:** `game/ambition_demo_mary_o/src/pipe.rs:226-267`
uses the shared `movement::transit_body` to move a body along its pipe, which
is correct. But it keeps the actor from steering by setting
`BodyCombat.recoil_lock_timer = TRANSIT_LOCK` each tick and sets the timer to
`0.0` on exit. This is not a recoil: a concurrent attack, capture or other
combat event may set a real recoil lock during transit, and pipe exit
unconditionally deletes it. The `a_pipe_ride_is_not_a_landing` production
tests check land SFX/dust, not this collision of authorities.

**Target:** a source-owned, temporary *input/action hold* evaluated at the
existing control gate. It is separate from who drives the body
(`DrivingParticipant`/`ControlClaims`), from movement pose ownership, and
from combat recoil. The pipe acquires its own hold for its own transit,
releases only that claim on exit, interruption, despawn or room teardown,
and reconstructs it deterministically after rewind. A combat hit during
the ride keeps its real recoil/hitstun. Two different holds can coexist;
ending one does not clear the other. Do not add another control gate, or
silently make `recoil_lock_timer` a multi-purpose lock.

**Witness:** enter a pipe with a non-zero recoil timer, let a hit or other
independent source increase recoil mid-ride, finish the pipe, and compare
against an identical control body without the pipe. The independent lock
must survive. Test early cancellation, restore, and two simultaneous bodies.
Poison: the old `= 0.0` on exit makes the first case fail. An engine
control-hold abstraction should have at least pipe plus one scripted
movement/cutscene consumer before widening its public authoring format.

## Acceptance

- `PossessionState` holds no brain state. Met.
- No exhaustive match has an arm for "a human is driving" beside arms for
  wanderers. Met: `Brain` has one variant, and `CharacterBrainTemplate`'s
  variants contain no player arm.
- A movement-only game's linked-crate count does not rise. Met, by not moving
  the brain data (see the decision above).
- A rewind across each control transition restores the same effective authority.
  Owed (see above).

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
