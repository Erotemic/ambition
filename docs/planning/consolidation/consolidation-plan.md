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
| 2 | C02 | Separate local lifetime/correlation identity from peer-stable mechanical provenance | **IS** the active campaign: [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity), nine of twelve roads closed (2026-09-16) | large | — (it is the campaign the others waited on; its own checkpoint is discharged) |
| 3 | C03 | Consolidate session-owned state and reduce reset-only App globals | **STARTABLE 2026-09-16 — every gate discharged** | large | ~~ID-PEER checkpoint~~ + ~~shell/content A-supersedes-B witness~~. Both discharged; the peer-identity one by its owner, [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity), which also names the one re-arm condition (`Q128`). |
| 4 | C04 | Make activated generation mechanics the only live-session construction source | candidate after session ownership stabilizes | medium | C03 owner decision + supported-composition decision. |
| 5 | C05 | Collapse live content/session publication onto one admitted candidate owner | **STARTABLE 2026-09-16 — every gate discharged, but RE-SCOPE FIRST: premise measured substantially stale the same day** | large | ~~Shell/content A-supersedes-B witness~~ + ~~identity checkpoint~~. Both discharged; the peer-identity one by its owner, [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity). |
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

**STATE:** ACTIVE, and it IS the campaign the other rows were told to wait for —
[ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity)
in the queue, **nine of twelve roads closed as of 2026-09-16**, three open and each
blocked outside this campaign (`Q122`, `Q128`, and the 25 unchecksummed float rows
which wait on netcode's N2).
**IMPLEMENTATION CAMPAIGN SIZE:** large
**DO NOT START BEFORE:** — ⛔ **THIS LINE USED TO READ "do not start before the
active identity campaign checkpoint", WHICH GATED THIS ROW ON ITSELF.** C02 is that
campaign. Corrected 2026-09-16; the per-road table, the arm holding each closure
and the discharge statement C03/C05 depend on all live in the queue row, not here.

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

**STATE:** STARTABLE 2026-09-16 — every gate discharged
**IMPLEMENTATION CAMPAIGN SIZE:** large
**DO NOT START BEFORE:** ~~A10 checkpoint~~ (discharged 2026-09-15) + ~~peer identity checkpoint~~ — **DISCHARGED 2026-09-16 by its owner.** The statement and its evidence live once, in [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity); this line is a pointer on purpose, because this file has already been bitten by a body that contradicted its own table (see C01). ⛔ It names one RE-ARM CONDITION: `Q128` rebases the simulation tick at an ACTIVATION moment, so if that road is ruled and started while this migration is in flight, coordinate rather than assume. ~~The shell/content A-supersedes-B race witness~~ — **DISCHARGED 2026-09-16**: both halves are now witnessed in the shipped composition, the session half by `a_candidate_session_replaced_while_pending_is_discarded` and the transaction half by `a_superseded_transaction_cannot_publish_in_the_shipped_app`, the latter poison-verified so its refusal is specific to supersession. `consolidation/README.md` carries the detail.

### CURRENT STATE

Source explicitly groups **36** App resources as gameplay-session or
activated-generation state. Activation reset is the correctness edge; retirement
cleanup is hygiene.

⚠ **THAT NUMBER WAS 32 UNTIL IT WAS RE-DERIVED 2026-09-16, AND THE DRIFT IS THE
CAMPAIGN'S OWN SUBJECT MOVING.** `SessionScopedResources` holds 29 `ResMut`
fields, not 25 — counted field by field in
`actor_monolith/src/session/teardown.rs`, each one a distinct App resource. The
four that arrived are `StocksMatchSettled`, `SuddenDeathEntered`, `LiveMatchTicks`
and `SessionMatchOrdinal`: the `MatchInstance`-stamped resources that ID-PEER
made MEMBERS of this grouping rather than moving elsewhere, which is the correct
outcome for them and grows C03's population by four. ⇒ **A campaign whose
starting census is four rows stale starts by consolidating a set it has not
enumerated.**

### INDEPENDENT TRUTHS INVOLVED

<!-- session-owner-census: SessionScopedResources=29 SessionOwnedCheckpointState=6 SessionMechanics=1 -->
⭐ **THE LINE ABOVE IS THE MACHINE-READABLE COPY AND
`scripts/check_session_owner_census_matches_source.py` COMPARES IT TO SOURCE.**
It exists because this census drifted by four while C03 waited on its gates, and
a number in prose has no way to notice that. The prose below is for readers; the
comment is for the guard, and the guard fails if they stop agreeing with
`teardown.rs` and `checkpoint.rs`.

`SessionScopedResources` (**29**, re-derived 2026-09-16 — the row said 25),
`SessionOwnedCheckpointState` (6, unchanged) and `SessionMechanics` (1 resource,
unchanged — it is ONE resource with six fields, and counting its fields is how
this total gets read as 41), plus `SessionRoot` as the current owner-scoped
model.

### WHY COMPLEXITY EXISTS

Much of this state was introduced as App resources for broad system access. Session ownership arrived later and now requires explicit reset and stale-retirement protection.

### WHAT COULD DISAPPEAR

⛔⛤ **MEASURED 2026-09-16: THE "RESET-ONLY" POPULATION IS EMPTY.** All 29
`SessionScopedResources` members have at least one reader outside the reset that
owns them — `scripts/measure_session_scoped_resource_readers.py`, which prints
its own patterns so a reader can see what it would miss. The thinnest are
`LastCutsceneRoom` (1 system param + 2 direct accesses) and `SwitchActivationQueue`
(2 + 2); the fattest are `ControlledSubject` (14 + 13) and `MovingPlatformSet`
(8 + 16). ⇒ **C03's session-scoped half is a MIGRATION, not a move.** There is no
cheap subset to lift out first, and a plan that opens by hunting for one will
spend its first day finding that out.

⚠ The counts are a FLOOR — the scan cannot see `SystemState`, an alias, or a
reader inside a macro — which is the right direction for THIS inference: a floor
proves a member HAS readers and can never prove one has none. ⛔ The first version
of that script reported three reset-only types and all three were wrong: it
matched `Res<Short>` after stripping each type to its last path segment, while
production writes `ResMut<ambition_cutscene::LastCutsceneRoom>`. A zero from a
name-matching scan is a claim about the QUERY.

⛔⛤ **AND "SEPARATE RESET LISTS" IS NOT A DUPLICATED AUTHORITY EITHER, MEASURED
2026-09-16.** There are TWO resource-reset lists — `SessionScopedResources` (29)
and `SessionOwnedCheckpointState` (6, all six checkpoint-operation types) — and
their intersection is **EMPTY**. Both run at `SessionScopeSet::Activate`. So they
are a PARTITION of session-owned state, not two copies of it: merging them buys
one fewer struct, not one fewer truth.

⚠ The activation and retire resets are ALSO not two lists. Both take the same
`SessionScopedResources` bundle and call the same `reset()`; the retire one is
declared HYGIENE at the site, and the activation one is what makes the next
session safe.

⚠ **AND `clear_transient_on_sandbox_reset` IS A DIFFERENT AXIS, not a third
list** — it takes `Commands` and entity `Query`s and no session resource at all.
⛔ A regex over its signature reported ZERO resource params, which is TRUE and
would have been reported as a finding by a scan that did not open it. It is a
correct zero for the wrong-sounding reason, which is exactly when to read the
function.

⇒ **WHAT IS LEFT FOR C03 ON THIS AXIS IS REPEATED OWNER GUARDS AND THE
`SessionRoot`-vs-App-global ownership question itself** — both of which are
migrations. Neither of the two cheap wins the row opened with survived
measurement. Do not force state that must exist before root creation into the
root.

⭐⭐ **AND THE "REPEATED OWNER GUARD" HAS A SHAPE, MEASURED 2026-09-16: IT IS ONE
GUARD SPELLED 185 TIMES, AND THERE ARE TWO SEMANTICS, NOT ONE.**

| spelling | what it is | production USES |
| --- | --- | ---: |
| `SessionWorldRef<T>` | `Single<Ref<T>, With<SessionRoot>>` | 163, in 91 files |
| `SessionWorldMut<T>` | `Single<&mut T, With<SessionRoot>>` | 22, in 14 files |
| `live_session_world_root` | finds the root whose scope equals the ACTIVE scope | 4, in 2 files |
| `session_root_for_scope` | finds a named scope's root, through the disabling marker | 11, in 6 files |

⚠ **THESE ARE USES, COMMENTS STRIPPED AND TESTS EXCLUDED.** An earlier version of
this table counted raw grep MENTIONS (177/29) and the prose read them as "sites",
which overstated the population by about 11%. ⛔ And a peer counting `Single<`
directly got **11** occurrences workspace-wide and could not reproduce any of it
— correctly, because these two are `pub type` ALIASES for `Single<..>`, so a scan
keyed on what they EXPAND TO cannot see a single one of their uses. Two honest
scans of one tree disagreed by an order of magnitude for that reason alone.

⛔ **THE TWO DISAGREE ONLY ON ONE FRAME, AND THAT IS WHY THIS IS SUBTLE.**
`Single` matches NOTHING when the count is not exactly one, and a system whose
`Single` fails is SILENTLY SKIPPED — so on a frame holding two roots, all 185
sites stop running while the four scope-aware ones resolve the live root
correctly. ⇒ The correctness of two hundred sites rests on an invariant that ONE
production arm asserts:
`the_shipped_app_never_holds_two_session_roots_across_a_handoff`
(`game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs:435`), which counts
roots every frame across a real shell handoff.

⚠ So the consolidation here is NOT "delete repeated guards" — they are one alias
used widely, which is already the consolidated form. It is deciding whether the
`Single` semantics or the scope semantics is the one this engine means, and the
answer changes what a handoff frame is allowed to look like. That is a ruling,
not a refactor, and it is FILED as `Q132` in
[`awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md) with the
three options costed. C03 should not move storage before it is answered.

### DEPENDENCIES / BLOCKERS

~~Finish A10 and peer identity first so the live/candidate session owner is stable.~~ **BOTH DISCHARGED** (A10 2026-09-15, peer identity 2026-09-16) — and that sentence is the reason the discharge is a claim about STABILITY and not about ID-PEER being finished, which it is not. Preserve rollback registrations. Mechanical edit admission is already established and is not a blocker.

### RISK

medium-high: moving rollback state or pre-root coordinator state to the wrong owner can break startup, restore, or snapshots.

### EXPECTED BENEFIT

Fewer independent process truths; session teardown becomes entity/owner retirement rather than global scrubbing; ownership is visible in storage.

## 4. C04 — Make activated generation mechanics the only live-session construction source

**STATE:** candidate after session ownership stabilizes. ⚠ **Its declared-profile half is MEASURED DELIVERED (2026-09-16); what remains is the composition-contract ruling.** Re-scope before costing.
**IMPLEMENTATION CAMPAIGN SIZE:** medium
**DO NOT START BEFORE:** C03 owner decision + supported-composition decision. `hold-ok` — this row genuinely delivers ONE half of its own scope (the declared-profile half, MEASURED 2026-09-16) and is STILL HELD on the other: the supported direct/headless composition contract is a ruling nobody has made, and C03's owner decision now depends on `Q132`.

### CURRENT STATE

`SessionMechanics` is the generation-owned construction input for live sessions. Direct/headless compositions can still fall back to App registries when no generation is activated.

### INDEPENDENT TRUTHS INVOLVED

`SessionMechanics`, App character/sheet/boss/developer registries, `GenerationMechanics`, direct composition profile.

### WHY COMPLEXITY EXISTS

Direct-entry compositions predate universal prepared-generation activation and still need a construction source.

### WHAT COULD DISAPPEAR

⛔⛤ **MEASURED 2026-09-16: THE "ACCIDENTAL MISSING-RESOURCE BRANCH" IS ALREADY A
DECLARED DECISION.** `GenerationMechanics::for_live_session(shell_routed, ..)`
REFUSES — returns `None` — when a shell-routed session has no activated
generation, and the discriminator is `SessionGatedSimulation`, which the source
describes as *"installed only by `ambition_game_shell`'s session plugin, never
inserted by direct-entry apps or headless harnesses"*. Composition MODE is asked,
not inferred. Every live-rebuild road (`session/reset/mod.rs`,
`room_transition/loading.rs`, `world/rooms/stage.rs`) goes through it; the
preparation road uses `GenerationMechanics::of`.

⚠ **AND THE ONE PRODUCTION CALLER OF THE UNREFUSING `new` IS CORRECT.**
`game/ambition_app/src/app/dev_runtime.rs:491` — the HOT RELOAD, which passes
`None` on purpose because it is BUILDING the generation that replaces the live
one, and says so at the call site. Narrowing `new` to `pub(crate)` was tried and
fails to compile for exactly that caller.

⇒ **WHAT IS LEFT FOR C04 IS THE RULING, NOT A REFACTOR** — *"decide supported
direct/headless composition contract"*, already its own DO-NOT-START-BEFORE. The
same shape C03's third candidate turned out to have. Keep explicit fixture
construction if still useful; the declared-profile half is delivered.

### DEPENDENCIES / BLOCKERS

Decide supported direct/headless composition contract; C03 owner placement; A10 live/candidate generation owner.

### RISK

medium: deleting fallback too early can break valuable demos/harnesses; retaining it implicitly can rebuild a session from current App values.

### EXPECTED BENEFIT

One generation authority for every live gameplay construction road; direct fixtures use an explicit prepared generation/profile.

## 5. C05 — Collapse live content/session publication onto one admitted candidate owner

**STATE:** STARTABLE 2026-09-16 — every gate discharged. ⛔ **BUT RE-SCOPE
BEFORE STARTING: its premise is substantially STALE, measured the same day.**
Five of the six values it proposes to collapse already land on ONE entity from
ONE lowering, and the sixth carries its value from the same frozen generation.
See CURRENT STATE.
**IMPLEMENTATION CAMPAIGN SIZE:** large **(as written; the measured remainder is
much smaller and has NOT been re-costed)**
**DO NOT START BEFORE:** ~~A10 complete~~ (discharged 2026-09-15) + ~~shell/content A-supersedes-B witness~~ (discharged 2026-09-16; both halves witnessed in the shipped composition, see `consolidation/README.md`) + ~~identity checkpoint~~ — **DISCHARGED 2026-09-16 by its owner**, stated once in [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity) with its evidence and its one re-arm condition (`Q128`, which rebases the tick at an activation moment).

### CURRENT STATE

`PreparedContentIdentity`, `ActiveContentBinding`, prepared content, LDtk index, generation mechanics, and room state do not all change under one current verdict. The content half has a gate/candidate road; the scene half is A10.

⚠ **PARTIALLY RE-DERIVED 2026-09-16, AND TWO OF THE SIX ARE NOT WHAT THIS ROW
IMPLIES.** `ActiveContentBinding` is a **Component**, not an App resource — one
declaration (`world/rooms/transaction.rs`) and ONE production insert site, onto
`session_root`, in the same `commands` batch that goes on to build the first
room's construction plan. So it is already carried by the session it describes
rather than queued as a separate global write. `PreparedContentIdentity` is also
a Component, and is DERIVED (`prepared_content.identity()`) rather than written.
⛔ The consolidation ledger had `ActiveContentBinding` recorded as a `Resource`,
marked SOURCE_CONFIRMED; that is corrected, and a rule now checks every item's
storage kind against source on each `--maintenance` run.

⇒ **ALL SIX ARE NOW RE-DERIVED, AND FIVE OF SIX ARE COMPONENTS.**

| the row's truth | the type | storage kind |
| --- | --- | --- |
| `PreparedContentIdentity` | `PreparedContentIdentity` | Component (DERIVED, `prepared_content.identity()`) |
| `ActiveContentBinding` | `ActiveContentBinding` | Component, inserted on `session_root`, ONE production site |
| prepared content | `PreparedContent` (`runtime/content_identity.rs`) | Component |
| LDtk index | `LdtkRuntimeIndex` | Component |
| generation mechanics | `SessionMechanics` | **Resource** — the only App global of the six |
| room state | `RoomSet`, `RoomGeometry` | Components (the ledger records `RoomSet` as *on SessionRoot*) |

⛔ **SO "SEPARATE QUEUED WRITES" DESCRIBES ONE VALUE, NOT SIX.** Five of the six
are already entity-carried; the collapse this row proposes is largely a collapse
that has happened.

⭐⭐ **AND THEY DO HANG OFF ONE ENTITY, FROM ONE LOWERING.**
`PlatformerSessionWorld` is a `#[derive(Bundle)]` of the session root's mutable
components — `catalogs`, `room_set`, `geometry`, `active_room`,
`starting_character`, `initial_body`, `requests` — and its own doc says it is
*"constructed only by lowering an immutable `PreparedPlatformerSource`"*. It is
built at `provider/src/lifecycle.rs:2166` as
`prepared_content.source().instantiate_live()`, in the same function that takes
`prepared_content.identity()` and, under the `ldtk` feature, installs the LDtk
index as *"a SEPARATE component on the same root by the road that installed the
format, so a game that uses no such format carries nothing for it"*.
`ActiveContentBinding` is inserted on that same `session_root`.

⇒ **ROOM STATE, PREPARED CONTENT, THE PREPARED IDENTITY, THE LDTK INDEX AND THE
CONTENT BINDING ALL LAND ON ONE ENTITY, DERIVED FROM ONE PREPARED SOURCE,
BEFORE PUBLICATION.** The row's premise — six values that "do not all change
under one current verdict" — is substantially stale.

⭐ **AND THE TWO SITES ARE THE SAME EDGE.** `provider/src/lifecycle.rs:2241`
calls `actor_monolith::session::setup::simulation_world(..)` — the function that
inserts `ActiveContentBinding` — from the SAME function that built the bundle at
`:2166`, passing `session_root: world`, the root that activation just spawned.
The comment at the call site states the design in the row's own words: *"Setup
publishes the session's content generation ON it — not into a process global that
a second session would have to overwrite."*

⚠ **WHAT IS GENUINELY LEFT IS `SessionMechanics`, AND EVEN IT IS NOT A LOOSE
WRITE.** It is a FIELD of the prepared content (`pub mechanical: SessionMechanics`,
`provider/src/lifecycle.rs:1526`), so its VALUE comes from the same frozen
generation — the doc there records why: a generation used to be prepared against
cast N and have its world built from N+1 with the identity still claiming N.
⛔ It is nonetheless a real App Resource: `Res`/`ResMut<SessionMechanics>` appears
in FOUR production files and `teardown.rs` reads it through `world.resource`. ⚠ I
did NOT locate its production install site — `insert_resource(..SessionMechanics)`
matches only a test — and after three name-matching scans produced confident zeros
tonight I am not reporting that absence as a finding. It is an open question for
whoever starts C05, not evidence.

⚠ Two caveats on the table. `PreparedContent` also names a non-ECS struct in
`ambition_content_pack`, so the Component is the runtime projection rather than
the only thing wearing that name. And a `RoomSet` deriving BOTH `Resource` and
`Component` exists in `tests/ambition_workspace_policy` — a policy fixture, not
production; a scan that counted it would report an App-global room set that does
not exist.

### INDEPENDENT TRUTHS INVOLVED

content candidate, shell activation gate, provider session world, room candidate, rollback contract, live content binding.

### WHY COMPLEXITY EXISTS

Content reload and room construction evolved as separate transactions and now meet at activation/reconstruction.

### WHAT COULD DISAPPEAR

Separate queued writes for values that all mean “this session is now generation N+1”. Keep projections that serve different APIs, but derive them from one published candidate.

### DEPENDENCIES / BLOCKERS

~~A10~~ (discharged 2026-09-15); ~~shell/content A-supersedes-B hold witness~~ (discharged 2026-09-16, both halves witnessed in the shipped composition); ~~peer-stable identity~~ (discharged 2026-09-16 by its owner). **No blocker remains.**

### RISK

high: half-published generation state can make simulation consume mechanics that do not match its identity/rollback contract.

### EXPECTED BENEFIT

One publication decision selects the candidate session/world. Content binding and prepared/session projections follow that owner.

## 6. C06 — Converge reconstruction entry roads on one materialization/publication engine

**STATE:** candidate; C01 is COMPLETE, so the gate is C05 alone. ⭐ **Premise SPOT-CHECKED 2026-09-16 and it HOLDS** — unlike the four rows re-derived the same day. See CURRENT STATE for what was and was not measured.
**IMPLEMENTATION CAMPAIGN SIZE:** large
**DO NOT START BEFORE:** ~~C01~~ (discharged 2026-09-15) + C05.

### CURRENT STATE

Initial session, room transition, same-room replay, checkpoint restore, New Game, and development reload already share parts of prepared room construction but still have different commit/publication wrappers.

⭐ **SPOT-CHECKED 2026-09-16 AND THIS ROW HOLDS — the first of five re-derived
today that did.** It is recorded because a run of stale rows makes the next one
look stale too, and that is how a correct row gets rewritten.

⇒ **THE MATERIALIZER IS ALREADY ONE PRIMITIVE.**
`RoomConstructionPlan::spawn_contents` has a single definition
(`world/rooms/stage.rs:379`) and exactly ONE production caller
(`session/setup.rs:189`); every other call site is inside that file's
`#[cfg(test)]` region. It delegates to `construct_room_candidate`, one exclusive
-world command that consults the opening decision before building anything.
`finalize_room_publication` likewise has one definition and two production call
sites.

⛔ **BUT THE WRAPPERS ABOVE IT ARE STILL DISTINCT, WHICH IS WHAT THIS ROW SAYS.**
The room-transition road has its own `RoomTransitionApply::stage(..)`
(`crates/ambition_platformer2d_runtime/src/room_transition/commit.rs:247`) with its own preflight —
`NoSessionWorld`, `SubjectCannotTransit { subject, missing }` — reached without
going through `session/setup.rs`. ⚠ I did NOT enumerate all six roads' wrappers;
what is measured is that the room MATERIALIZER is shared and at least one commit
wrapper is genuinely separate. The row's premise survives on that evidence, and
its size has NOT been re-derived.

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

**STATE:** candidate after composition decision. ⚠ **Its census was re-derived 2026-09-16 with the repository's OWN instrument: 726/196, essentially flat** (the baseline is 732/196). ⛔ An earlier version of this line claimed +16% from a scan of mine that counted inline `#[cfg(test)]` modules — see CURRENT STATE.
**IMPLEMENTATION CAMPAIGN SIZE:** medium
**DO NOT START BEFORE:** Supported composition profiles must be named first.

### CURRENT STATE

Static source contains **726** optional Res/ResMut occurrences over **196**
unique type spellings. Most are not defects. Three high-authority cases already
use composition discriminators: session scope, generation mechanics, and content
binding.

⛔⛤ **THIS LINE SAID 732/196, THEN SAID "at least 850" FOR AN HOUR ON
2026-09-16, AND BOTH OF THOSE WERE MINE TO GET RIGHT.** I wrote my own scan,
counted `Option<Res<` openings across tracked `.rs` excluding only `/tests/`
directories and `tests.rs` files, got 850, and reported the population had GROWN
~16%. ⇒ **The repository already has the instrument this row cites**, and running
it — `python3 scripts/architecture_census.py` — reports **726 occurrences over
196 unique types TODAY**. Against the baseline's 732/196 that is essentially
FLAT, six DOWN, not 118 up.

⚠ **THE WHOLE DIFFERENCE IS THE TEST BOUNDARY.** The census strips each file from
its first `#[cfg(test)]` onward and scans only `crates/`, `game/` and `tools/`;
my scan kept inline `#[cfg(test)]` modules. Those 124 occurrences are real code
and they are not what this row is about. **ASK THE TOOL, DO NOT MODEL IT** — and
when a number disagrees with a baseline by 16%, suspect the instrument before the
tree.

⭐ **AND THE CENSUS CROSS-CHECKS SOMETHING ELSE TONIGHT:** it independently
reports *"explicit process resources with session/generation semantics: 36"*,
which is C03's re-derived count, measured by a different road than the
field-by-field read that produced it.

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

**STATE:** later cleanup; A10 no longer blocks it
**IMPLEMENTATION CAMPAIGN SIZE:** medium
**DO NOT START BEFORE:** ~~A10~~ (discharged 2026-09-15) — but do not run during another large ownership migration. ID-PEER is one: stay off session/canonical identity.

### CURRENT STATE

The responsibility map and facade crates include compatibility re-exports/mirrors. Some are useful public ergonomics; others hide which lower crate owns a concept.

⭐⭐ **MEASURED 2026-09-16, AND THE SHAPE IS NARROWER THAN "facade crates"
PLURAL: IT IS ONE CRATE.** Counting `pub use` STATEMENTS in production files
(a floor on ITEMS — `pub use foo::{A, B, C}` counts once):

| package | `pub use` | of which CROSS-CRATE |
| --- | ---: | ---: |
| `ambition_platformer2d` | 187 | **168** |
| `ambition_platformer2d_actor_monolith` | 109 | 14 |
| `ambition_platformer2d_runtime` | 52 | 31 |
| `ambition_platformer2d_core` | 44 | 6 |
| `ambition_combat` | 34 | 15 |
| (workspace total) | 937 | 300 |

⇒ **`ambition_platformer2d` IS THE FACADE: 90% of its re-exports cross a crate
boundary, and it holds 168 of the workspace's 300 — 56% in ONE package.** The
remaining 132 are spread across ~79 others, which is background, not a campaign.
A row that says "facade crates include compatibility re-exports" reads as a
diffuse problem; it is one crate plus a long tail.

⚠ This counts STATEMENTS and says nothing about which are deliberate public
ergonomics — the row's own distinction, and the part no scan can make. What it
buys is the denominator: whoever takes C08 is triaging ~168 lines in one file
tree, not auditing a workspace.

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

The workspace has 80 packages (verified against `cargo metadata` 2026-09-16, and
now guarded — see the ledger check). The largest package has **108,646** nonblank
Rust lines, but durable architecture explicitly rejects a size-only carve of
`actor_monolith`.

⛔⛤ **AND RE-DERIVING THAT NUMBER 2026-09-16 FOUND SOMETHING THE HEADLINE HIDES:
IT MIXES SOURCE AND TESTS, AND FOR A CRATE-BOUNDARY CAMPAIGN THAT IS THE WHOLE
QUESTION.**

| package | src | tests | total |
| --- | ---: | ---: | ---: |
| `crates/ambition_platformer2d_actor_monolith` | 60,228 | 48,418 | 108,646 |
| `game/ambition_app` | 25,581 | 79,942 | 105,523 |
| `game/ambition_content` | 41,594 | 10,895 | 52,489 |
| `crates/ambition_combat` | 30,842 | 21,270 | 52,112 |
| `crates/ambition_platformer2d_core` | 28,054 | 18,151 | 46,205 |

⇒ `actor_monolith`'s PRODUCTION source is 60,228 lines — 55% of the headline —
and `game/ambition_app` is the second-largest package almost entirely because of
its test suite (76% tests; it holds the `app_it` integration arms). **You do not
carve a package because its integration tests are large**, and a size-ordered
list that mixes the two puts a test crate second. ⚠ The row already rejects a
size-only carve; this says the size itself was not the size anyone meant.

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

Start **C03: session-owned state and reset-infrastructure consolidation**. ✔ Both
gates it named are discharged — A10 on 2026-09-15 and the peer-identity checkpoint
on 2026-09-16 — so this is no longer "only after"; it is the recommendation.
~~Close the named shell/content A-supersedes-B supersession witness first~~ —
**DISCHARGED 2026-09-16**, both halves witnessed in the shipped composition; see
`README.md`. Mechanical editor admission does not block this campaign; its shared
protocol is already the baseline.

⛔ **BUT DO NOT MOVE STORAGE BEFORE `Q132` IS ANSWERED.** It asks whether a
handoff frame holding two session roots should make 185
`Single<.., With<SessionRoot>>` sites run or skip, and that ruling decides
whether such a frame may exist at all — which is precisely what moving
session-owned storage determines. Measuring and sequencing may start now; moving
may not.

⚠ **AND TWO OF THIS SEQUENCE'S OWN PREMISES WERE MEASURED AWAY ON 2026-09-16**
(see C03's CURRENT STATE): there is no reset-only subset to lift out, and the two
reset lists are a disjoint partition rather than two copies. The owner-by-owner
sequence below is still the right shape; the cheap first win it implies is not
there.

Do not begin by moving all 36 values.
Use a bounded owner-by-owner sequence:

1. Re-run `python3 scripts/architecture_census.py` and confirm the explicit narrower-lifetime list.
2. For each family, state whether the value must exist before `SessionRoot`, only during a live session, or only for presentation.
3. For rollback-registered values, record the current registration and restore boundary before changing storage.
4. Select one coherent family with one owner. `SessionOwnedCheckpointState` is a good first review unit because source already declares its six values as one gameplay-session coordinator. This is a review starting point, not a pre-decided move to one component.
5. Choose storage from semantics: `SessionRoot` component/bundle for live-session state; explicit session-keyed process coordinator when pre-root availability is required; ordinary App resource only when process lifetime is real.
6. Remove the old reset/retirement compensation only after the new owner is the sole authority.
7. Keep direct Bevy query/system-parameter use. Do not add a generic state-container abstraction to hide ECS.

**Exit property:** a future reviewer can name one owner for each migrated fact. Session activation no longer needs to overwrite a process-global copy only to make the next session safe.
