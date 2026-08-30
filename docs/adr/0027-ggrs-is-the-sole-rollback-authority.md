# ADR 0027: GGRS is the sole rollback authority

## Status

**Accepted; implemented for the simulation harness** (2026-07-18).

- **Supersedes:** Ambition-owned ephemeral snapshot/restore machinery described by the earlier N3 planning state

## Decision

Ambition uses `ggrs` and `bevy_ggrs` as the sole authority for ephemeral rollback:

- GGRS owns input synchronization, prediction, frame history, save/load requests,
  rollback-window selection, resimulation, confirmed-frame tracking, and sync-test
  checksum comparison.
- `bevy_ggrs` owns Bevy world snapshots, rollback entity creation/destruction,
  component/resource restoration, and allocator-local `Entity` remapping.
- Ambition owns only the typed registration contract, deterministic domain codec /
  checksum projections, the input bridge, exact prepared-content/schema identity,
  and session invalidation policy.

The deleted `ambition_platformer2d_runtime::snapshot` subsystem is not retained behind a
compatibility facade. Persistence/checkpoint serialization, when required, will
be a separate product boundary and must not become a second rollback engine.

## Identity and ownership

`bevy_ggrs::RollbackId` is GGRS's frame-history identity. `SimId` remains
Ambition's semantic authored/runtime identity for construction, diagnostics,
relationships, replay, observations, and future persistence. A `SimId` does not
by itself opt a presentation-only entity into rollback; authoritative family
anchors install `Rollback` explicitly.

A GGRS session is bound to the exact `PreparedContentIdentity` and deterministic
rollback-registration fingerprint present when it starts — specifically, the
identity carried by the canonical `SessionRoot` of the gameplay session that
owns the timeline. A changed content epoch or registration schema invalidates
and removes the active session before another GGRS frame can run. LDtk hot
reload therefore cannot commit while a rollback session is active; a coordinated
session restart is required.

⚠ **The contract resolved its content with a global first match until
2026-08-30**, which made "the prepared identity" whichever one the archetype
happened to yield. It is now looked up on the root the contract's own scope
owns, so an entity from another activation can neither satisfy it nor be
mistaken for its subject.

## Three lifetimes

```text
Process
  └── Gameplay session      (SessionScopeId)
        └── Rollback timeline (RollbackTimelineGeneration)
```

The middle one was missing, and the word "session" was doing both of the inner
two. The symptom: quit a Smash match to the title, start Ambition, and the
player could move but no door would open. Retiring Smash's scope removed the
canonical root its still-installed timeline was policing; the contract read that
as an illegal mid-session content disappearance and invalidated the timeline;
and the invalidation was a process-global value the next game read as its own,
so room-transition commit refused every transition forever.

**The rule, written once in `ActiveRollbackAuthority::installed`:**

```text
previous.owner == incoming.owner   ->  the diagnosis CARRIES
previous.owner != incoming.owner   ->  a FRESH authority
```

- Health **may** carry between rollback generations of one gameplay session: a
  desync must not launder itself by restarting or rebasing GGRS (AC23).
- Health **must not** carry between gameplay sessions: nothing session A's
  timeline discovered is a fact about session B's world.
- Rollback authority is owned by a `SessionScopeId`. `ActiveRollbackAuthority`
  holds the owner, the timeline generation, the contract, and the status as one
  resource, because splitting them is what let a status outlive the contract
  that produced it.
- Every read names a scope. `confirmation_for(scope)` answers `Unavailable` to
  any scope the authority does not govern — never `Unhealthy`. "Not mine" and
  "mine and broken" are different answers: the first resolves on the next
  install, the second never resolves. Gameplay reads through
  `SessionRollbackConfirmation`, which has no way to reach the state without
  naming the live scope.
- Historical diagnostics are process lifetime and have **no** gameplay
  authority. `RollbackDiagnosticHistory` remembers "Smash desynced because …"
  after Smash has ended, and is allowed to precisely because it can no longer
  block anything. Nothing may gate work on it.
- Session cleanup is required for hygiene; **scope ownership is what protects
  correctness**. A retired scope's authority is removed, and a survivor is inert
  anyway because it names an owner and the next scope is not it.

Deliberate retirement is not corruption. `enforce_session_contract` asks *whose
world is this* before it asks anything else: a timeline whose scope is no longer
live stands down. Only for the live scope do "the schema changed" and "the
content disappeared" mean what they say — that check is not weakened.

Retirement is also ordered, in `SessionScopeSet::RetireAuthority`, which sits
between `Presentation` and `Cleanup` so an authority stands down before the
world it governs is removed. That ordering is hygiene: the ownership rule above
is what holds when scheduling regresses, and the tests are built to prove which
is which.

### The guarantee, and how it is guarded

| Test | Proves |
|---|---|
| `shell_host_lifecycle::a_smash_session_does_not_take_ambitions_doors_with_it` | The acceptance walk: Smash → title → Ambition, and a real room transition commits. Runs in both admissible orderings of the local-session owner and the shell bridge. |
| `shell_host_lifecycle::the_full_multi_game_lifecycle_is_leak_free_under_rollback` | The whole multi-game walk under `SimulationHost::Rollback`, not only `RenderFrame`. |
| `session_ownership_tests::a_poisoned_session_hands_nothing_to_the_next_one` | A genuinely invalidated scope A, then scope B, with **no reset in between**. |
| `session_ownership_tests::a_stale_authority_left_allocated_is_inert_for_the_next_session` | A's poisoned authority deliberately left in the world; B is unaffected because the owner does not match. |
| `session_ownership_tests::a_rebase_without_retiring_the_scope_keeps_its_poison` | AC23 survives: same scope, new timeline, the diagnosis still refuses. |
| `session_ownership_tests::retiring_a_scope_is_teardown_not_corruption` | Retirement with the timeline still installed records no failure. |
| `session_ownership_tests::a_contract_resolves_only_the_root_its_own_scope_owns` | Two roots present; B binds B's, and content vanishing from B's own root is still caught. |
| `teardown::tests::activating_a_session_clears_what_a_skipped_teardown_left_behind` | The same guarantee for the eleven session-scoped process globals. |

## Session-scoped process globals

The same class of bug lives wherever a resource mirrors one live session. The
eleven in `ambition_platformer2d_actor_monolith::session::teardown` were reset
**only** at retirement, and nine of them change the next session's behaviour if
that reset is delayed, misordered, or skipped: a dangling possessed-body handle,
a `specs_loaded` latch that suppresses the next session's repopulation, a
room-transition cooldown that refuses doors, a buffered interact nobody pressed.

They are now re-established on `SessionScopeActivated`, in
`SessionScopeSet::Activate`, before any provider builds the world that reads
them. That is ownership without an accessor check at any of the several hundred
read sites: **the value a session reads is one its own activation wrote.**
Retirement reset stays, and is documented as hygiene.

## Registration policy

Authoritative mutable components/resources use one of:

- an explicit canonical byte strategy, also used for checksums;
- an exact clone strategy plus an explicit canonical checksum projection;
- an exact clone strategy for immutable/structural shell data whose behavior is
  already bound by prepared-content identity.

Allocator-local relationships use `MapEntities`. Frame-derived values are
registered as derived and rebuilt by their ordinary maintenance systems. Sim
message buffers are cleared on `LoadWorld`; replayed inputs regenerate the
accepted future. Presentation/external side effects must later be released only
from confirmed frames.

Registration names, kinds, type identities, and policy details form an
order-independent, versioned schema fingerprint. Conflicting duplicate names
fail during App construction, and so do two different types whose identities
collide.

⚠ **the fingerprint deliberately hashes no organisational label.** As accepted
this sentence also listed OWNERS, and the type identity was the whole
`std::any::type_name` — so which module registered a thing, and which crate and
module a type lived in, were wire-format facts. Both were removed as the same
mistake: schema v5 (2026-07-31) dropped `owner`, and v20 (2026-08-09) narrowed
the type identity to the type's final segment, because a carve moves a type's
crate AND its module path while changing nothing a peer can observe. What
remains hashed is the stable name, the kind, the type's own name, and the policy
detail. The duplicate-identity refusal above is what keeps the narrower form
sound.

## Harness and networking sequence

The simulation harness uses `SyncTestSession` first: real game inputs drive the
real `GgrsSchedule`, and GGRS repeatedly saves, loads, resimulates, and compares
checksums. Future native/Matchbox P2P hosts construct another GGRS `Session` and
install it through the same exact-content/schema seam; transport does not alter
simulation ownership.

`GgrsSchedule` uses Bevy's single-threaded executor. Ambition's explicit phase
sets define the semantic ordering; systems intentionally unordered inside one
phase use stable same-build App construction order. Bevy's exhaustive ambiguity
diagnostic is disabled only for `GgrsSchedule` because emitting hundreds of
pairwise edges would duplicate that phase architecture without strengthening the
same-build contract. `SyncTestSession` remains the behavioral determinism oracle:
it repeatedly restores and re-executes the actual schedule and rejects divergent
checksums.

## Consequences

- Thousands of lines of custom history, blob dispatch, room staging for rollback,
  entity reconciliation, compatibility preflight, and restore tests are deleted.
- Ordinary construction/transition/reset remain canonical game architecture, but
  no longer masquerade as rollback implementation.
- The next rollback-networking slice is confirmed-frame side-effect quarantine
  plus a Matchbox-backed two-peer handshake. The independent construction-plan
  track remains the next world-construction milestone.


## Current implications for agents

- Put authoritative gameplay systems in `GgrsSchedule`; never add another rollback driver or frame-history store.
- Register every mutable authoritative component/resource through `AmbitionRollbackApp`, including entity remapping where needed.
- Use `SyncTestSession` for deterministic rollback verification before adding transport.
- Keep presentation and irreversible host effects outside speculative execution until the confirmed-frame effect boundary is implemented.
- Keep `SimId` as semantic identity and let `bevy_ggrs::RollbackId` remain an internal frame-history identity.
- Read rollback confirmation through `SessionRollbackConfirmation`, never a bare resource. If you find yourself adding "…and check it belongs to this session" at a call site, the authority is the thing to fix.
- Never clear rollback health as a step in going home, launching a game, or restarting GGRS. `ActiveRollbackAuthority::installed` is the only place that decides, and `acknowledge_and_clear` is the only deliberate escape.
- A process-global resource that mirrors one live session belongs in `SessionScopedResources`, which re-establishes it at activation. Do not rely on retirement having run.
