# Architecture consolidation plan

- **Census baseline:** `662a9b56096a304ce9fcfe3ebcf1177403f2452d`
- **Planning-control refresh:** `2dbd81abc50f42a601d8e6177478ef0985002365`
- **Purpose:** remove independent truths after the current architecture milestone. The planning-control row was refreshed by the documentation consolidation; other ranked architecture claims retain the census baseline. This file does not authorize work on active A10 or identity campaigns.

The ranking is by authority/lifecycle leverage, not by LOC.
A candidate can move down if a new source inspection shows that two values have distinct semantic owners.

## Priority table

| Rank | ID | Opportunity | State | Campaign size | Do not start before |
| --- | --- | --- | --- | --- | --- |
| 1 | C01 | Finish A10 as the one live room/session replacement transaction | **COMPLETE 2026-09-15.** Post-A10 demolition is the active lane | large | — |
| 2 | C02 | Separate local lifetime/correlation identity from peer-stable mechanical provenance | ACTIVE separate identity campaign | large | Do not start before the active identity campaign checkpoint. |
| 3 | C03 | Consolidate session-owned state and reduce reset-only App globals | RECOMMENDED first new campaign; A10 half of its gate is met | large | ID-PEER checkpoint — C03 touches `SessionRoot`, activation and provenance, the neighbourhood ID-PEER is changing. Plus the shell/content A-supersedes-B race witness. |
| 4 | C04 | Make activated generation mechanics the only live-session construction source | candidate after session ownership stabilizes | medium | C03 owner decision + supported-composition decision. |
| 5 | C05 | Collapse live content/session publication onto one admitted candidate owner | candidate; A10 is complete, so the remaining gates are the other two | large | Shell/content A-supersedes-B witness + identity checkpoint. |
| 6 | C06 | Converge reconstruction entry roads on one materialization/publication engine | candidate; C01 is complete, so the gate is C05 | large | C05. |
| 7 | C07 | Replace optional canonical-authority fallbacks with explicit composition contracts where the authority is required | candidate after composition decision | medium | Supported composition profiles must be named first. |
| 8 | C08 | Prune compatibility facades and forwarding mirrors after canonical owners settle | later cleanup; A10 no longer blocks it | medium | Do not run during a large ownership migration. ID-PEER is one; stay off session/canonical identity. |
| 9 | C09 | Review crate boundaries by semantic ownership, not size | later structural review | medium-large | After owner consolidation, not before. |
| 10 | C10 | Restore planning control-plane separation between current state and history | **COMPLETED 2026-09-14** | small-medium | Closed by semantic-preservation documentation cleanup. |

## 1. C01 — Finish A10 as the one live room/session replacement transaction

**STATE:** COMPLETED 2026-09-15. Post-A10 demolition is the active lane.

⛔⛤ **THIS SECTION DESCRIBED THE PRE-A10 WORLD UNTIL 2026-09-16**, three weeks
after the work landed — it said *"candidate entity support exists, but the normal
room bracket is still off"* and `STATE: ACTIVE`, while the priority table two
screens above already said COMPLETE. A ranked plan whose body contradicts its own
table is a second authority, and the body is the half a reader acts on.

There is ONE road into a live session and ONE publication authority per level.
Every road that can change the authoritative world — room transition, reset,
death reconstruction, shell handoff, dev LDtk reload — crosses its own
publication's verdict, each with a refusal arm and an admission control in
`app_it`. A verified publication also FREEZES what it owes the world outside its
own population, so a room published inside a pending candidate session announces
nothing to the live one until that session is admitted.

**What disappeared, as promised by the row:** the destructive retire-before-verify
road; the candidate mode flag (`ROOM_CANDIDATE_BRACKET` is DELETED, not frozen
on, so there is no second road to drift onto); the split hot-reload writes; and
room-verifier responsibility for a world it cannot retain beside N.

**REGRESSION RULE:** a new road that changes the authoritative world must cross a
publication verdict. ⛔ Do not reintroduce a flag that selects between a
candidate road and a live one — a mode flag is how twenty-seven construction arms
came to certify a road the game no longer used.

⇒ Owner document: [construction and reconstitution](../engine/construction-and-reconstitution.md).
Do not use this row as an implementation plan for the demolition lane; use it to
find roads whose replacement now exists.

## 2. C02 — Separate local lifetime/correlation identity from peer-stable mechanical provenance

**STATE:** ACTIVE separate identity campaign
**IMPLEMENTATION CAMPAIGN SIZE:** large
**DO NOT START BEFORE:** Do not start before the active identity campaign checkpoint.

### CURRENT STATE

`ContentEpoch`, `SessionScopeId`, and `ShellActivationId` are useful local identities, but current provenance paths feed them into values that can affect canonical world identity. `PreparedContentIdentity` and `TransactionId` mix local and canonical jobs.

### INDEPENDENT TRUTHS INVOLVED

local lifecycle tokens, content fingerprints/schema, `SimId`, room plan identity, transaction provenance, shared match/session identity.

### WHY COMPLEXITY EXISTS

Local stale-event/lifetime control and peer equality were represented by some of the same aggregate values.

### WHAT COULD DISAPPEAR

Peer-visible dependence on prior local reload/session/navigation history; mixed-responsibility identity wrappers.

### DEPENDENCIES / BLOCKERS

Active peer-stable identity campaign; coordinate with A10 provenance before making new candidate transaction identity final.

### RISK

high: identity mistakes can create deterministic worlds that compare unequal or unequal worlds that compare equal.

### EXPECTED BENEFIT

Local tokens stay local. Canonical provenance uses only peer-stable mechanical facts.

## 3. C03 — Consolidate session-owned state and reduce reset-only App globals

**STATE:** RECOMMENDED first new campaign after current milestone
**IMPLEMENTATION CAMPAIGN SIZE:** large
**DO NOT START BEFORE:** A10 checkpoint + peer identity checkpoint. The shell/content A-supersedes-B race witness should also be closed before touching activation plumbing.

### CURRENT STATE

Source explicitly groups 32 App resources as gameplay-session or activated-generation state. Activation reset is the correctness edge; retirement cleanup is hygiene.

### INDEPENDENT TRUTHS INVOLVED

`SessionScopedResources` (25), `SessionOwnedCheckpointState` (6), and `SessionMechanics` (1), plus `SessionRoot` as the current owner-scoped model.

### WHY COMPLEXITY EXISTS

Much of this state was introduced as App resources for broad system access. Session ownership arrived later and now requires explicit reset and stale-retirement protection.

### WHAT COULD DISAPPEAR

Reset-only process storage where direct `SessionRoot` ownership works; repeated owner guards; separate reset lists. Do not force state that must exist before root creation into the root.

### DEPENDENCIES / BLOCKERS

Finish A10 and peer identity first so the live/candidate session owner is stable. Preserve rollback registrations. Mechanical edit admission is already established and is not a blocker.

### RISK

medium-high: moving rollback state or pre-root coordinator state to the wrong owner can break startup, restore, or snapshots.

### EXPECTED BENEFIT

Fewer independent process truths; session teardown becomes entity/owner retirement rather than global scrubbing; ownership is visible in storage.

## 4. C04 — Make activated generation mechanics the only live-session construction source

**STATE:** candidate after session ownership stabilizes
**IMPLEMENTATION CAMPAIGN SIZE:** medium
**DO NOT START BEFORE:** C03 owner decision + supported-composition decision.

### CURRENT STATE

`SessionMechanics` is the generation-owned construction input for live sessions. Direct/headless compositions can still fall back to App registries when no generation is activated.

### INDEPENDENT TRUTHS INVOLVED

`SessionMechanics`, App character/sheet/boss/developer registries, `GenerationMechanics`, direct composition profile.

### WHY COMPLEXITY EXISTS

Direct-entry compositions predate universal prepared-generation activation and still need a construction source.

### WHAT COULD DISAPPEAR

The second live-construction source when the product moves to a universal generation path. Keep explicit fixture construction if still useful, but make it a declared profile instead of an accidental missing-resource branch.

### DEPENDENCIES / BLOCKERS

Decide supported direct/headless composition contract; C03 owner placement; A10 live/candidate generation owner.

### RISK

medium: deleting fallback too early can break valuable demos/harnesses; retaining it implicitly can rebuild a session from current App values.

### EXPECTED BENEFIT

One generation authority for every live gameplay construction road; direct fixtures use an explicit prepared generation/profile.

## 5. C05 — Collapse live content/session publication onto one admitted candidate owner

**STATE:** candidate after A10 and shell/content supersession closure
**IMPLEMENTATION CAMPAIGN SIZE:** large
**DO NOT START BEFORE:** A10 complete + shell/content A-supersedes-B witness + identity checkpoint.

### CURRENT STATE

`PreparedContentIdentity`, `ActiveContentBinding`, prepared content, LDtk index, generation mechanics, and room state do not all change under one current verdict. The content half has a gate/candidate road; the scene half is A10.

### INDEPENDENT TRUTHS INVOLVED

content candidate, shell activation gate, provider session world, room candidate, rollback contract, live content binding.

### WHY COMPLEXITY EXISTS

Content reload and room construction evolved as separate transactions and now meet at activation/reconstruction.

### WHAT COULD DISAPPEAR

Separate queued writes for values that all mean “this session is now generation N+1”. Keep projections that serve different APIs, but derive them from one published candidate.

### DEPENDENCIES / BLOCKERS

A10; shell/content A-supersedes-B hold witness; peer-stable identity.

### RISK

high: half-published generation state can make simulation consume mechanics that do not match its identity/rollback contract.

### EXPECTED BENEFIT

One publication decision selects the candidate session/world. Content binding and prepared/session projections follow that owner.

## 6. C06 — Converge reconstruction entry roads on one materialization/publication engine

**STATE:** candidate after C01/C05
**IMPLEMENTATION CAMPAIGN SIZE:** large
**DO NOT START BEFORE:** C01 + C05.

### CURRENT STATE

Initial session, room transition, same-room replay, checkpoint restore, New Game, and development reload already share parts of prepared room construction but still have different commit/publication wrappers.

### INDEPENDENT TRUTHS INVOLVED

room plan, lifecycle intent, checkpoint pinned inputs, new-game policy, generation candidate, session retention policy.

### WHY COMPLEXITY EXISTS

The roads have legitimate policy differences, but construction and publication infrastructure grew at different times.

### WHAT COULD DISAPPEAR

Duplicate commit wrappers and special hot-reload construction. Keep only policy-specific input/admission layers above one normal candidate materializer.

### DEPENDENCIES / BLOCKERS

C01 and C05. Checkpoint/new-game retention semantics must remain explicit.

### RISK

medium-high: over-consolidation can erase valid retention/admission differences.

### EXPECTED BENEFIT

One construction/publication primitive with explicit policy inputs; fewer roads that can drift on identity, validation, or retirement.

## 7. C07 — Replace optional canonical-authority fallbacks with explicit composition contracts where the authority is required

**STATE:** candidate after composition decision
**IMPLEMENTATION CAMPAIGN SIZE:** medium
**DO NOT START BEFORE:** Supported composition profiles must be named first.

### CURRENT STATE

Static source contains 732 optional Res/ResMut accesses over 196 type spellings. Most are not defects. Three high-authority cases already use composition discriminators: session scope, generation mechanics, and content binding.

### INDEPENDENT TRUTHS INVOLVED

`SessionGatedSimulation`, `ActiveSessionScope`, `SessionMechanics`, `ActiveContentBinding`, mechanical admission, readiness, LDtk capability.

### WHY COMPLEXITY EXISTS

One codebase supports shell, direct, headless, fixture, and optional-capability compositions.

### WHAT COULD DISAPPEAR

Bare absence that means both “capability not installed” and “required authority went missing”. Keep legitimate absence as a named profile/capability.

### DEPENDENCIES / BLOCKERS

C03/C04; public composition campaign/A9 when scheduled.

### RISK

medium: making every resource required would reduce supported composition and fight Bevy; leaving required production state optional fails open.

### EXPECTED BENEFIT

Each optional canonical authority has one explicit reason for absence, and production profiles cannot represent a live state without required owners.

## 8. C08 — Prune compatibility facades and forwarding mirrors after canonical owners settle

**STATE:** later cleanup
**IMPLEMENTATION CAMPAIGN SIZE:** medium
**DO NOT START BEFORE:** Do not run during A10 or another large ownership migration.

### CURRENT STATE

The responsibility map and facade crates include compatibility re-exports/mirrors. Some are useful public ergonomics; others hide which lower crate owns a concept.

### INDEPENDENT TRUTHS INVOLVED

facade modules, legacy paths, re-exports, lower owner crates.

### WHY COMPLEXITY EXISTS

Crate decomposition and public API migration preserved old paths while owners moved.

### WHAT COULD DISAPPEAR

Internal mirror paths with no supported API role. Keep a deliberate public facade if it improves use without becoming a second owner.

### DEPENDENCIES / BLOCKERS

Owner boundaries from C03-C07; consumer/API census.

### RISK

medium-low mechanically, but broad source churn can obscure active architecture work.

### EXPECTED BENEFIT

Imports name the owner internally; public facade surface is deliberate and small enough to describe.

## 9. C09 — Review crate boundaries by semantic ownership, not size

**STATE:** later structural review
**IMPLEMENTATION CAMPAIGN SIZE:** medium-large
**DO NOT START BEFORE:** After owner consolidation, not before.

### CURRENT STATE

The workspace has 80 packages. The largest package has 104,962 nonblank Rust lines, but durable architecture explicitly rejects a size-only carve of `actor_monolith`.

### INDEPENDENT TRUTHS INVOLVED

high-fan-in runtime/actor/shared crates, small capability crates, forwarding/facade crates.

### WHY COMPLEXITY EXISTS

Ambition has both real capability boundaries and migration residue. LOC and dependency count cannot distinguish them alone.

### WHAT COULD DISAPPEAR

Only boundaries that mostly forward one tightly coupled owner or preserve historical split with no current semantic responsibility. Conversely, split a large package only at a proven independent owner.

### DEPENDENCIES / BLOCKERS

C03-C08 settle owner map; use full manifest/API/consumer census from helper.

### RISK

medium: a wrong merge increases dependency fanout; a wrong split adds wiring and compile churn.

### EXPECTED BENEFIT

Package graph follows ownership/change boundaries and preserves useful independent capabilities.

## 10. C10 — Restore planning control-plane separation between current state and history

**STATE:** COMPLETED 2026-09-14

The documentation consolidation at source snapshot `2dbd81abc50f` returned the
live control plane to the repository's existing contract:

- `queue.md` contains open executable work rather than completion diaries;
- `status.md` is a short orientation snapshot;
- `awaiting-maintainer-decision.md` contains unresolved choices only;
- closed/retracted investigation history is left to Git;
- two large current owner docs were rewritten around current authority/open work
  instead of review chronology.

The stable ledger entry `TRANS-PLANNING-HISTORY` remains as the closure receipt so
future census updates do not rediscover the same debt under a new paragraph.

**REGRESSION RULE:** if a live control-plane file starts accumulating closed case
files again, delete/compress the history in place. Do not create another archive
document inside `docs/`.

## Recommended first new campaign after the current architecture milestone

Start **C03: session-owned state and reset-infrastructure consolidation** only after A10 and the active peer-identity checkpoint are stable.
Close the named shell/content A-supersedes-B supersession witness first if it still shares activation code at that point.
Mechanical editor admission does not block this campaign; its shared protocol is already the baseline.

Do not begin by moving all 32 values.
Use a bounded owner-by-owner sequence:

1. Re-run `python3 scripts/architecture_census.py` and confirm the explicit narrower-lifetime list.
2. For each family, state whether the value must exist before `SessionRoot`, only during a live session, or only for presentation.
3. For rollback-registered values, record the current registration and restore boundary before changing storage.
4. Select one coherent family with one owner. `SessionOwnedCheckpointState` is a good first review unit because source already declares its six values as one gameplay-session coordinator. This is a review starting point, not a pre-decided move to one component.
5. Choose storage from semantics: `SessionRoot` component/bundle for live-session state; explicit session-keyed process coordinator when pre-root availability is required; ordinary App resource only when process lifetime is real.
6. Remove the old reset/retirement compensation only after the new owner is the sole authority.
7. Keep direct Bevy query/system-parameter use. Do not add a generic state-container abstraction to hide ECS.

**Exit property:** a future reviewer can name one owner for each migrated fact. Session activation no longer needs to overwrite a process-global copy only to make the next session safe.
