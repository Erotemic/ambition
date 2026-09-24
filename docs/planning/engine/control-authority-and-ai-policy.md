# Control authority and AI policy are two facts in one component

**Owner: the engine.** This page owns the split between who drives a body
(control authority) and what the body's own brain would do (AI policy). The
first slice of that split is built. What remains is the A4 regrouping (see the
[frontier](actor-monolith-work-frontier.md)) and one owed rewind witness.

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
- **Temporary control is a claim, arbitrated in one place.** `ControlClaims` in
  `shared_tangle::temporary_control` holds one named `Option<SimId>` field per
  `ControlClaimant` (`Possession`, `Mount`). Domains file and drop claims;
  `project_control_claims` decides the winner, ordered after
  `PlayerSimulationSet::Possession` and `CombatSet::Settle`. Readers ask
  `ControlClaims::holds(..)` directly. Precedence is the enum's order, in one
  place. The claims are rollback state (`actor.control_claims`).
- **Named fields, not a collection:** two claimants exist, precedence is total,
  and a fixed struct clones without allocation or ordering ambiguity. A third
  claimant is a field and a match arm that the compiler makes you visit.
- **Release reveals the remaining winner.** `drop_claim` clears one field and the
  projection re-reads what is left. A dead mount ends the ride's claim; it does
  not make the body autonomous.
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
| `TemporaryControl` projection | `shared_tangle::temporary_control` | the claim arbiter only |
| `ControlledSubject` | `shared_tangle::markers` | `possession.rs`, `session/teardown.rs` |

`session/teardown.rs` is a lifetime edge, not a second authority. A census of
"who writes this" cannot tell those apart by itself.

Three of the four types are floor- or domain-owned already, and the writers
concentrate in two monolith files. `PossessionState` is filed under
`abilities/traversal/`, which makes possession look like a traversal ability.
The remaining carve is not "extract control": name the transition authority and
move it out of the ability tree, so that `possession.rs` and
`control/authority.rs` are one domain with a module path that says so. That is
A4's regrouping.

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

## Brain data stays in `ambition_characters` (decided 2026-09-02)

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
