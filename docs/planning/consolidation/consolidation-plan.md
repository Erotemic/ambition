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
| 2 | C02 | Separate local lifetime/correlation identity from peer-stable mechanical provenance | **IS** the active campaign: [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity), which re-derives its own road count from its table — this cell deliberately states none, having carried "nine of twelve" while the owner said fourteen of seventeen | large | — (it is the campaign the others waited on; its own checkpoint is discharged) |
| 3 | C03 | Consolidate session-owned state and reduce reset-only App globals | **STARTABLE 2026-09-16 — every gate discharged** | large | ~~ID-PEER checkpoint~~ + ~~shell/content A-supersedes-B witness~~. Both discharged; the peer-identity one by its owner, [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity), which also names the one re-arm condition (`Q128`). |
| 4 | C04 | Make activated generation mechanics the only live-session construction source | candidate after session ownership stabilizes | medium | C03 owner decision + supported-composition decision. |
| 5 | C05 | Collapse live content/session publication onto one admitted candidate owner | **STARTABLE 2026-09-16 — every gate discharged, premise measured substantially stale, and RE-COSTED to small the same day: the authority collapse has already happened and the remainder is one value's storage kind with no defect behind it** | small (was large) | ~~Shell/content A-supersedes-B witness~~ + ~~identity checkpoint~~. Both discharged; the peer-identity one by its owner, [ID-PEER](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity). |
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
in the queue. ⛔ **THE ROAD COUNT IS NOT RESTATED HERE.** It read "nine of twelve"
while the owner row said fourteen of seventeen, which is the failure this whole
plan exists to remove: a summary of a live table is a copy corrections do not
reach. What is durable and worth stating is the SHAPE — every open road is
blocked outside this campaign (`Q122`, `Q128`, and the unchecksummed float rows,
which wait on netcode's N2), so nothing here is in flight.
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

Source explicitly groups **37** App resources as gameplay-session or
activated-generation state. Activation reset is the correctness edge; retirement
cleanup is hygiene.

⚠ **THAT NUMBER WAS 32 UNTIL IT WAS RE-DERIVED 2026-09-16, AND THE DRIFT IS THE
CAMPAIGN'S OWN SUBJECT MOVING.** `SessionScopedResources` holds 30 `ResMut`
fields, not 25 — counted field by field in
`actor_monolith/src/session/teardown.rs`, each one a distinct App resource. The
four that arrived are `StocksMatchSettled`, `SuddenDeathEntered`, `LiveMatchTicks`
and `SessionMatchOrdinal`: the `MatchInstance`-stamped resources that ID-PEER
made MEMBERS of this grouping rather than moving elsewhere, which is the correct
outcome for them and grows C03's population by four. ⇒ **A campaign whose
starting census is four rows stale starts by consolidating a set it has not
enumerated.**

### INDEPENDENT TRUTHS INVOLVED

<!-- session-owner-census: SessionScopedResources=30 SessionOwnedCheckpointState=6 SessionMechanics=1 -->
⭐ **THE LINE ABOVE IS THE MACHINE-READABLE COPY AND
`scripts/check_session_owner_census_matches_source.py` COMPARES IT TO SOURCE.**
It exists because this census drifted by four while C03 waited on its gates, and
a number in prose has no way to notice that. The prose below is for readers; the
comment is for the guard, and the guard fails if they stop agreeing with
`teardown.rs` and `checkpoint.rs`.

`SessionScopedResources` (**30**, re-derived 2026-09-16 — the row said 25, then 29),
`SessionOwnedCheckpointState` (6, unchanged) and `SessionMechanics` (1 resource,
unchanged — it is ONE resource with six fields, and counting its fields is how
this total gets read as 41), plus `SessionRoot` as the current owner-scoped
model.

### WHY COMPLEXITY EXISTS

Much of this state was introduced as App resources for broad system access. Session ownership arrived later and now requires explicit reset and stale-retirement protection.

### WHAT COULD DISAPPEAR

⛔⛤ **MEASURED 2026-09-16, RE-RUN 2026-09-17: THE "RESET-ONLY" POPULATION IS
EMPTY.** All 30 `SessionScopedResources` members have at least one reader outside
the reset that owns them — `scripts/measure_session_scoped_resource_readers.py`, which prints
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
2026-09-16.** There are TWO resource-reset lists — `SessionScopedResources` (30)
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

⭐⭐ **AND THE "REPEATED OWNER GUARD" HAS A SHAPE: IT IS ONE GUARD SPELLED ~183
TIMES, AND THERE ARE TWO SEMANTICS, NOT ONE.**

⛔⛤ **THE SIZE IS NOT OWNED HERE.** `BEVY-SESSION-ROOT` in
[`architecture-census.md`](architecture-census.md) is the one owner of this
population's total and prints four methods beside it. This table carries only
the SHAPE — the per-spelling split and the two semantics — re-measured
2026-09-19 under that row's method so the two cannot disagree.

<!-- alias-split: SessionWorldRef=167/91 SessionWorldMut=16/13 live_session_world_root=3/1 session_root_for_scope=2/2 -->
| spelling | what it is | production uses |
| --- | --- | ---: |
| `SessionWorldRef<T>` | `Single<Ref<T>, With<SessionRoot>>` | 167, in 91 files |
| `SessionWorldMut<T>` | `Single<&mut T, With<SessionRoot>>` | 16, in 13 files |
| `live_session_world_root` | finds the root whose scope equals the ACTIVE scope | 3, in 1 file |
| `session_root_for_scope` | finds a named scope's root, through the disabling marker | 2, in 2 files |

⚠ **THE METHOD IS THE PARAMETER FORM: `Name<` for the aliases and `name(` for
the functions**, over `crates/` + `game/` with test files dropped,
`#[cfg(test)]` modules stripped and comments stripped. The two alias rows sum to
the census row's **183 in 95 files** by construction, which is the point of
saying the method out loud.

⛔⛤ **AND THE PREVIOUS VERSION OF THIS TABLE MIXED TWO METHODS INSIDE ITSELF,
FOUND 2026-09-19.** It read `163/91`, `22/14`, `4/2` and `11/6` under a header
saying *"comments stripped and tests excluded"*. The alias rows were that; the
helper rows were not — `session_root_for_scope`'s `6 files` is the ALL-FILES
count (17 raw mentions in 6 files today), while production code-only is 2 in 2.
⇒ A header that states one method for a table whose rows were gathered by two is
worse than no header, because it stops the next reader checking. An earlier
version before that counted raw grep MENTIONS (177/29) and the prose read them
as "sites", overstating the population by about 11%. ⛔ And a peer counting `Single<`
directly got **11** occurrences workspace-wide and could not reproduce any of it
— correctly, because these two are `pub type` ALIASES for `Single<..>`, so a scan
keyed on what they EXPAND TO cannot see a single one of their uses. Two honest
scans of one tree disagreed by an order of magnitude for that reason alone.

⛔ **THE TWO DISAGREE ONLY ON ONE FRAME, AND THAT IS WHY THIS IS SUBTLE.**
`Single` matches NOTHING when the count is not exactly one, and a system whose
`Single` fails is SILENTLY SKIPPED — so on a frame holding two roots, every one
of those alias sites stops running while the scope-aware ones resolve the live
root correctly. ⇒ The correctness of two hundred sites rests on an invariant that ONE
production arm asserts:
`the_shipped_app_never_holds_two_session_roots_across_a_handoff`
(`game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs:535`), which counts
roots every frame across a real shell handoff.

⚠ So the consolidation here is NOT "delete repeated guards" — they are one alias
used widely, which is already the consolidated form. It was deciding whether the
`Single` semantics or the scope semantics is the one this engine means, and the
answer changes what a handoff frame is allowed to look like. That was a ruling,
not a refactor, and it was filed as `Q132`.

⭐ **DECIDED 2026-09-19: the `Single` semantics is the meaning.** There is
exactly one canonical live `SessionRoot`; a two-root frame is invalid rather
than skippable. ⇒ The alias sites are correct as written, the scope-aware
helpers are for lifecycle code that legitimately sees both sides of a handoff,
and `unique_session_world_root`'s `assert!` states the invariant rather than
guessing at it. **The consolidation left here is to say that out loud in one
place**, so the next author picks a spelling on purpose rather than by import —
which was option (c)'s one good part, and the ruling makes it a documentation
job rather than a choice.

### STEP 3 IS DONE FOR ONE FAMILY: THE CHECKPOINT COORDINATOR'S ROLLBACK PARTITION

The sequence's step 3 says *"for rollback-registered values, record the current
registration and restore boundary before changing storage."* MEASURED 2026-09-16
across every `.rs` in `crates/` and `game/`, over all **ten** `rollback_resource_*`
registration methods the `RollbackRegistrar` trait declares — 68 raw occurrences,
every one classified as a state call site, an entity-mapping call site, or the
trait's own forwarding definition, yielding **54 state-registered resource types**
workspace-wide — for `SessionOwnedCheckpointState`'s six members:

| member | rollback key | boundary |
| --- | --- | --- |
| `SessionCheckpointOperations` | `resource.session_checkpoint_operations` | rewinds; must NOT reset at a room rebase |
| `SessionCheckpointOutcomes` | `resource.session_checkpoint_outcomes` | rewinds; read frames later by the startup road |
| `AcceptedCheckpointRestore` | `resource.accepted_checkpoint_restore` | rewinds; spans frames by design |
| `OutstandingCheckpointRequest` | `resource.outstanding_checkpoint_request` | rewinds; kept until the slot admits it |
| `SessionStartupResume` | `resource.session_startup_resume` | rewinds; key renamed when the VALUE changed |
| `AbandonedCheckpointOperation` | **none, deliberately** | written from `Update`, which never rewinds |

⛔ **THE PARTITION IS 5 + 1, AND SOURCE'S OWN DOC BLOCK SAID FOUR.** The four was
a count of the BULLETS above it, one of which pairs a registered value
(`AcceptedCheckpointRestore`) with an unregistered one
(`AbandonedCheckpointOperation`) because they cooperate. ⇒ A prose list grouped by
COLLABORATION reads as a count of the MECHANISM, and an auditor who took the
number would have carried one host-side value into a rollback migration. Source is
corrected and states the five keys.

⭐ **THE SIXTH'S ABSENCE IS THE DESIGN, NOT AN OMISSION.**
`AbandonedCheckpointOperation` carries a VALUE-COMPLETE note — key, admitted
frame and the accepted operation's checksum — precisely so a world that rewound
past its branch DISCARDS it (`AbandonmentVerdict::Stale`) rather than cancelling a
healthy replacement that reused the rewound sequence number. Registering it would
make a local preparation failure, which two peers need not agree about, into
shared state. ⇒ **C03 must not "finish the family" by registering it.**

⛔⛤ **AND THE FIRST VERSION OF THAT MEASUREMENT ASKED ABOUT ONE METHOD OF TEN.**
It scanned only `rollback_resource_clone_checksum` while printing the verdict *"NO
rollback registration"* — a claim its query could not support. The checkpoint
family's answer did not change, because all five do use that method, so it was
right BY LUCK: one resource over, `PendingLifecycleCommit` is documented as
rollback-registered and does not appear under that name. ⇒ **When a scan's verdict
is a NEGATIVE, the method list is the finding's real subject.** Widening it also
surfaced two classification errors that a narrow query hid: a doc comment in
`teardown.rs` NAMING a registration is a raw occurrence that is not a call (strip
the comment REGION first — recognising the prose is the rule backwards), and four
resources carry BOTH a state registration and a `map.resource.*` entity-mapping
one, which is two registrations of different KINDS, not two authorities.

⭐ `scripts/check_session_owner_census_matches_source.py` RULE 3 now holds this:
every member registers under a key the doc block NAMES, or source declares the
absence with the words `DELIBERATELY NOT REGISTERED` beside it. The exemption is
derived from source rather than listed in the guard, so the review that adds a
member is where the decision gets recorded. Poison-verified three ways in the
shipped files, restored by md5.

### AND FOR THE BIG FAMILY: 24 OF `SessionScopedResources`' 30 ARE ROLLBACK STATE

RE-DERIVED 2026-09-17 with the same widened scan, and the partition is three ways
rather than two: **`SessionScopedResources` holds 30, of which 24 are
state-registered, 2 declare themselves DERIVED (`ControlledSubject`,
`EncounterView`) and 4 carry no rollback decision of any kind
(`BossEncounterRegistry`, `CutsceneTriggerQueue`, `CutsceneAdvanceRequest`,
`CutsceneSkipHold`).** ⚠ It read 22 / 3 / 4 of 29 on 2026-09-16; the move is one
new member plus `AuthoredOccurrences`, which stopped being declared derived and
is registered now (`f15461f52`) — not two more registrations landing. ⇒ **This is the number that prices C03**, and it says the campaign
is mostly a rollback-state migration rather than a storage tidy-up: step 3 applies
to three quarters of the population, and step 6 ("remove the old compensation only
after the new owner is the sole authority") has a wire-format identity attached to
each of those 24.

⚠ **A NARROW SCAN SAID SIX.** The first pass of this measurement asked about
`rollback_resource_clone_checksum` only and reported 6 registered / 23 not —
which would have priced this campaign as a storage move with a few rollback
values attached, the opposite of the truth. Same defect as the checkpoint family's
"four", four times larger.

⭐ **ONE KEY DELIBERATELY DOES NOT MATCH ITS TYPE AND IT IS NOT A DEFECT.**
`RoomTransitionCooldown` registers as `resource.sandbox_sim_state`. Source says
why, beside it: *"THE STABLE NAMES DO NOT MOVE … identities on the wire; the
schema fingerprint deliberately excludes owner labels so an ownership repoint is
not a wire-format event."* ⇒ C03 must not "tidy" a rollback key to match a type
name. A key is an identity two peers agree on, not a label.

**Seven are not state-registered — and THREE OF THOSE SEVEN ALREADY HAVE THEIR
ANSWER WRITTEN IN SOURCE.** `ControlledSubject`, `EncounterView` and
`AuthoredOccurrences` each call `declare_rollback_derived_resource`, which is a
recorded decision that the value is RECOMPUTED rather than restored, not an
omission. ⇒ The registrar has THREE verdicts, not two — registered, declared
derived, and silent — and only the third is an open question.

⚠ **I WROTE "SEVEN NEED READING" ONE COMMIT AGO AND THAT WAS A THIRD NARROW
QUERY IN THE SAME AFTERNOON.** I had grepped the `rollback_resource_*` methods
and not the `declare_rollback_derived_*` ones, so a deliberate decision read as an
absence. ⭐ `ControlledSubject`, the one this row flagged as suspicious because it
holds an `Option<Entity>`, is precisely one of the three with an answer: it is the
body driven by the primary **LOCAL** control authority, which is host-side by
definition and must not rewind.

⇒ **FOUR CARRY NO ROLLBACK DECISION OF ANY KIND**, and reading them answered two:
`BossEncounterRegistry` is an authored read-only catalog behind a one-shot latch
and `CutsceneSkipHold` is HUD-only by its own doc, both correctly unregistered.
**The remaining two — `CutsceneTriggerQueue` and `CutsceneAdvanceRequest` — are
written or consumed INSIDE the rewinding schedule and rewind with nothing.** That
is now its own queue row,
[CUTSCENE-ROLLBACK-DECISION](../queue.md#cutscene-rollback-decision--two-session-scoped-cutscene-values-cross-into-simulation-with-no-rollback-decision),
because it is open executable work rather than a census fact, and C03 does not own
it: the decision may be to move `CutsceneAdvanceRequest` onto the control frame
rather than to register it.

⛔ **THE GUARD DELIBERATELY DOES NOT ENFORCE THIS YET.** Extending RULE 3's
"register or declare" rule from the checkpoint family to `SessionScopedResources`
would go red on these four the moment it landed, which is a fail-closed check
pausing a working repository over a question nobody has answered. ⇒ Answer the
four first, then widen the rule; that ordering is the point.

### STEP 2, ANSWERED FOR THE WHOLE AGGREGATE: THE RESET RUNS BEFORE THE ROOT EXISTS

MEASURED 2026-09-16 from `SessionScopeSet`'s own declaration and
`reset_session_scoped_resources_on_activation`'s doc: `Activate` is *"a newly live
scope re-establishes the process-global state that mirrors one session, BEFORE any
provider builds that session's world"*, and the ordering is held across three
crates — `ambition_game_shell` chains
`(GameplaySessionSet::Bridge, SessionScopeSet::Activate, GameplaySessionSet::Providers)`
in `Update`, `adopt_candidate_platformer_session` is in `Providers`, and
`maintain_local_session` starts GGRS only when `session_world_entity(world).is_some()`.

⇒ **SO THE ANSWER TO STEP 2 IS THE SAME FOR EVERY MEMBER OF BOTH BUNDLES, AND
IT IS NOT ONE OF THE THREE OPTIONS THE STEP OFFERS.** ⛤ This read *"all 35
values"* until 2026-09-19, which was `29 + 6` — the sum of the two bundle counts
taken before `SessionScopedResources` gained its thirtieth member. A DERIVED
total is the drift surface nobody guards: `check_session_owner_census_matches_source.py`
RULE 5 holds every restatement of `30`, of `6` and of the `37` total across the
corpus, and a hand-added `35` is none of those. ⇒ Stated as the membership it
means, so there is no third number to keep. They are not "needed before `SessionRoot`
exists" in the sense of being READ there; they are WRITTEN there, and only because
the storage outlives the session. ⭐⭐ **A value stored on `SessionRoot` needs no
activation reset at all: a freshly built root carries fresh components BY
CONSTRUCTION.** The reset edge is not a thing to relocate — **it is a thing the
migration DELETES**, and it exists purely as compensation for process-global
storage.

⇒ That reframes C03's step 6. *"Remove the old reset/retirement compensation only
after the new owner is the sole authority"* reads like cleanup; it is the campaign's
actual deliverable, and the size of the prize is two systems plus the correctness
argument that holds them: `SessionScopedResources`' 30 + 6 exhaustively
destructured fields whose only job is to undo the previous session.

⛔ **TWO CONSTRAINTS THE MIGRATION INHERITS, both already written beside the code.**

1. **The rollback-mutator answer would change.** 24 of `SessionScopedResources`' 30 are rollback-registered
   and the reset is an ordinary `Update` system; it is currently legal because *"these
   writes land before there is a frame zero to rewind to"*. Storage on a root built
   inside the provider step sits on the OTHER side of that boundary. ⇒ Re-derive the
   guard's answer per value; do not assume the current one travels.
2. **It rests on the A10 candidate staying HIDDEN.** A10.5 prepares the session world
   while the route is still pending, so a root for the incoming scope EXISTS before the
   reset runs — it carries `InactiveCandidate`, a Bevy disabling component, so
   `session_world_entity`'s default query cannot see it. That is HELD by
   `a_hidden_candidate_session_is_invisible_to_the_live_world_and_visible_to_its_transaction`,
   not assumed. ⚠ **And this is the same frame Q132 was about** — two roots exist, one
   hidden.

⭐⛤ **AND THE RULING (2026-09-19) RATIFIES EXACTLY THIS SHAPE RATHER THAN
DISTURBING IT.** Preparing a replacement while the current session stays live is
ALLOWED, and the incoming candidate must carry a distinct candidate/prepared
identity and must not masquerade as a `SessionRoot`. `InactiveCandidate` is that
distinct identity, and the arm above is the witness that it does not read as a
canonical root. ⇒ The hidden candidate is not a two-root frame in the sense the
ruling forbids — it is the prescribed lifecycle, and the correctness edge for
these values is therefore fixed rather than contingent: **exactly one root is
ever visible, so "before the root exists" means what it already meant.**

⚠ What this obliges instead is a POSITIVE one: a candidate must never count as a
canonical root in any witness of the invariant. That is a requirement on the
arms, and it is recorded in the campaign's witness list rather than left implicit
in the query that happens to filter it.

### DEPENDENCIES / BLOCKERS

~~Finish A10 and peer identity first so the live/candidate session owner is stable.~~ **BOTH DISCHARGED** (A10 2026-09-15, peer identity 2026-09-16) — and that sentence is the reason the discharge is a claim about STABILITY and not about ID-PEER being finished, which it is not. Preserve rollback registrations. Mechanical edit admission is already established and is not a blocker.

### RISK

medium-high: moving rollback state or pre-root coordinator state to the wrong owner can break startup, restore, or snapshots.

### EXPECTED BENEFIT

Fewer independent process truths; session teardown becomes entity/owner retirement rather than global scrubbing; ownership is visible in storage.

## 4. C04 — Make activated generation mechanics the only live-session construction source

**STATE:** candidate after session ownership stabilizes. ⚠ **Its declared-profile half is MEASURED DELIVERED (2026-09-16); what remains is the composition-contract ruling.** Re-scope before costing.
**IMPLEMENTATION CAMPAIGN SIZE:** medium
**DO NOT START BEFORE:** the supported-composition decision (`Q144`). `hold-ok` — this row genuinely delivers ONE half of its own scope (the declared-profile half, MEASURED 2026-09-16) and is STILL HELD on the other: the supported direct/headless composition contract is a ruling nobody has made. ⭐ **`Q132`'S HALF OF THIS GATE IS DISCHARGED (2026-09-19)** — C03's owner decision no longer waits on it, so `Q144` is now this row's ONLY maintainer hold.

⛔⛤ **AND THE `Q132` RULING NARROWS WHAT `Q144` IS ALLOWED TO ANSWER, WHICH MAKES THIS ROW CHEAPER RATHER THAN MORE EXPENSIVE.** The scoping rule says App-global mutable state is appropriate only where simultaneous sessions would legitimately share exactly the same value, and the ruling adds explicitly: *do not preserve ambiguous fallback behaviour merely for old direct-entry tests.* The App-registry fallback this row exists to remove is App-global mutable construction input that two coexisting sessions could legitimately differ on — so it is on the wrong side of the rule ALREADY, independent of how `Q144` rules on composition. ⇒ What `Q144` still owns is whether direct entry must ACTIVATE a prepared generation or may declare its inputs another explicitly-scoped way; it no longer owns whether the anonymous App-global fallback may stay.

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
`game/ambition_app/src/app/dev_runtime.rs:525` — the HOT RELOAD, which passes
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
**IMPLEMENTATION CAMPAIGN SIZE:** ⭐ **RE-COSTED 2026-09-16: SMALL, AND POSSIBLY
NOT OWED.** The row was written as large. Five of its six values are already
entity-carried on one session root from one lowering (see CURRENT STATE), so the
whole remainder is `SessionMechanics`, the one App resource — and its AUTHORITY
is already single: `PreparedCandidateSession::adopt` installs it at adoption and
nowhere else, `reset_session_scoped_resources_on_retire` removes it, and it is
in `SessionScopedResources`' census as one member. ⇒ What is left is a STORAGE
KIND change for one value, not an authority collapse.

⚠ **THE COST, MEASURED, WITH THE COMMAND:** `grep -rn "SessionMechanics"
--include=*.rs crates/ game/ examples/ | grep -v tests` gives **34 production
lines**, of which **4 are `Res` reads, 1 installs, 1 removes** and the rest are
the type, its fields, a re-export and doc references. So ~6 behavioural sites.
⛔ **And moving it makes those four readers WORSE, not better** — every one is
`Option<Res<SessionMechanics>>` today and would become a session-root lookup, in
crates that read it precisely because they should not have to find the root.
⇒ **Nothing here is motivated by a measured defect.** Starting C05 should begin
by deciding whether the storage change buys anything, not by scheduling it.
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

⭐ **AND THE TWO SITES ARE THE SAME EDGE.** `provider/src/lifecycle.rs:2251`
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

**STATE:** candidate after composition decision. ⚠ **Its census is 806 occurrences over 203 type spellings (`architecture_census.py`, 2026-09-19), after the instrument was twice found miscounting its own corpus** — see CURRENT STATE, which now records that the row's two previous figures (732/196 and 726/196) and the hand scan that contradicted them were all wrong in different ways.
**IMPLEMENTATION CAMPAIGN SIZE:** medium
**DO NOT START BEFORE:** Supported composition profiles must be named first — **FILED 2026-09-19 as [`Q146`](../awaiting-maintainer-decision.md), which this gate had been naming without a question number for four days.** ⚠ **ONE INSTANCE OF THAT DECISION IS FILED SEPARATELY AS `Q144`** — whether every supported composition must activate a prepared generation, or direct entry keeps the App-registry road — and it is C04's whole remaining scope. It does NOT settle this row: C07 needs the general profile vocabulary across every optional-authority spelling in the tree (203 of them, see CURRENT STATE), and `Q144` rules on one family.

⭐ **`Q132`'S SCOPING RULE (2026-09-19) SHRINKS THIS ROW WITHOUT CLOSING IT.** The rule decides, for any given optional authority, whether an App-global fallback may exist at all — that is the per-value test this campaign lacked. What it does not decide is which compositions the engine PROMISES to support, and "required" means "required in a profile". ⇒ `Q146` is now the whole of this gate rather than the vaguer "name the profiles".

⭐⛤ **THE `Q132` RULING ASKED FOR ONE SPECIFIC INSPECTION — *"places where
App-global fallback state exists only because session identity was OPTIONAL"* —
AND ITS ENTRY POINT IS MEASURED, 2026-09-19.** Over production source with
comments and test modules stripped, `Option<Res<..>>` / `Option<ResMut<..>>` occurs
**714 times over 189 distinct type LEAVES**, and the distribution is not flat:

⚠ **714 IS NOT THE 806 ABOVE, AND THE DIFFERENCE IS THE INSTRUMENT RATHER THAN
THE TREE.** `architecture_census.py` owns the campaign figure and counts more
spellings of "optional authority" than the bare `Option<Res<T>>` shape scanned
here; this pass also folds a qualified path onto its LEAF so two spellings of
one type count once. ⇒ Read 806/203 as the population and the table below as a
DISTRIBUTION over it, not as a competing total. Neither number supersedes the
other and the census tool stays the owner.

| optional authority | occurrences | files |
|---|---|---|
| `ActiveSessionScope` | **56** | 38 |
| `GameAssets` | 29 | 17 |
| `Assets` | 24 | 12 |
| `UserSettings` | 20 | 13 |
| `PreparedCharacterRegistry` | 18 | 13 |
| `ActiveMatch` | 17 | 14 |
| `SimTick` | 14 | 11 |

⇒ **`ActiveSessionScope` IS THE POPULATION THE RULING NAMES**, by a factor of
two over anything else: 56 production sites take SESSION IDENTITY ITSELF as
optional, so each carries a branch for "there is no session", and what that
branch reads is precisely the question. ⚠ This is an ENTRY POINT and not a
defect list — most are legitimately optional (a presentation system that simply
does nothing without a session is not App-global fallback state). The triage is
per site: does the `None` arm READ or WRITE state that a second coexisting
session could legitimately differ on?

⚠ **AND TWO OTHER ROWS ARE INTERESTING FOR REASONS THE RULING STATES
EXPLICITLY.** `UserSettings` (20) is named in the ruling as a SEPARATE
AUTHORITY that must not silently become live simulation state — an optional
read of it inside the simulation is the exact shape to check. `SimTick` (14) is
a simulation clock, which the ruling lists among the things that should be
scoped.

⭐⭐ **AND A SHARPER CUT THAN FREQUENCY, MEASURED 2026-09-19: INTERSECT THE
OPTIONAL READS WITH THE STATE THE TREE ALREADY DECLARES SESSION-OWNED.** The
distribution above ranks by how OFTEN a type is optional; this asks instead
whether the engine has already said the type belongs to a session. Taking
`C03`'s 37 session-owned members (`SessionScopedResources` 30 +
`SessionOwnedCheckpointState` 6 + `SessionMechanics`) and intersecting with
production `Option<Res<..>>` / `Option<ResMut<..>>` reads gives **11 of the 37,
across 45 sites**:

| session-owned member | optional sites | files |
|---|--:|--:|
| `ControlledSubject` | 13 | 10 |
| `MintedItemBaseline` | 5 | 3 |
| `OccurrenceBaseline` | 5 | 3 |
| `CustodyBaseline` | 5 | 3 |
| `AuthoredOccurrences` | 5 | 3 |
| `BaseGravity` | 4 | 2 |
| `SessionMechanics` | 3 | 3 |
| `ActiveConversation` | 2 | 2 |
| `StocksMatchSettled`, `AcceptedCheckpointRestore`, `MovingPlatformSet` | 1 each | 1 each |

⇒ **THIS IS THE POPULATION WHERE THE TRIAGE QUESTION IS SHARPEST**, because the
`None` arm is reading past a DECLARED session owner rather than past an unknown
one. ⛔⛤ **AND THE FIRST CANDIDATE PICKED OUT OF IT WAS THE OPPOSITE OF WHAT IT
LOOKED LIKE, WHICH IS WHY THE TRIAGE IS PER SITE AND NOT PER TYPE.**
`BaseGravity` is rollback state read optionally by the kaleidoscope menu app
(`game/ambition_app/src/menu/kaleidoscope_app.rs:603`, *"for the row's direction
label"*) — rollback state, a menu composition, session-owned member: every
surface feature of a defect. Following it to its consumer settles it the other
way. The value is passed as `Option<&BaseGravity>` into `dev_toggles`, whose
whole use is
`ctx.base_gravity.map_or("n/a", |g| g.direction_label())` — the `None` arm
renders a placeholder string. It reads no substitute, writes nothing, and
manufactures no simulation state. ⇒ **A LEGITIMATE optional read, and the
clearest example on this page of the rule the row already states**: the
question is what the `None` arm DOES, and a type's pedigree cannot answer it.

⛔ **SO THIS IS TRIAGE INPUT, NOT A DEFECT LIST, AND `Q146` IS WHY.** What makes
a site adjudicable is knowing whether its composition is one the engine promises
to support and must therefore declare this authority, or a presentation surface
that may legitimately show nothing. Until the profiles are named, the 45 sites
are a population to walk, not work to start. ⚠ The intersection is keyed on the
same `Option<Res<T>>` spelling as the table above and inherits its blind spot,
below.

⛔ **WHAT THIS MEASUREMENT IS NOT.** It is keyed on the `Option<Res<T>>`
SPELLING, so it cannot see a fallback expressed another way — and one is
already known to be missed by it: `GenerationMechanics::for_live_session`'s
`shell_routed == false && active.is_none()` branch reaches the App registries
without any optional resource in its signature. A first, narrower scan for
`scope`/`session` used with `is_none`/`unwrap_or` found only **6** production
sites and missed that one too, because the variable is called `active`. ⇒ Treat
714/189 as one lens on this population rather than its census, which is `C07`'s
own job and is gated on `Q146`.


### CURRENT STATE

Static source contains **806** optional Res/ResMut occurrences over **203**
unique type spellings — `scripts/architecture_census.py`, re-run 2026-09-19, and
it is the owner of this figure. Most are not defects. Three high-authority cases
already use composition discriminators: session scope, generation mechanics, and
content binding.

⛤ **THIS SENTENCE SAID `820 / 206` UNTIL 2026-09-19 WHILE ITS OWN BODY, TWENTY
LINES DOWN, ALREADY SAID `806 / 203`.** The correction was written and the
headline was not updated with it — a body contradicting its own opening, which
is the exact failure this file records for C01 and then repeated. The history
below is intact and is why the number moved; the opening now carries the
current one.

⛔⛤ **THIS ROW HAS NOW CARRIED FOUR FIGURES AND THE THIRD ONE WAS THE
INSTRUMENT'S.** It said 732/196, then "at least 850" for an hour on 2026-09-16,
then 726/196 with the note *"ASK THE TOOL, DO NOT MODEL IT"* — and on 2026-09-17
the tool turned out to be cutting each file from its FIRST `#[cfg(test)]` to the
end. In this tree a module declares its tests near the TOP
(`#[cfg(test)] mod tests;`), so everything below that line was invisible:
`architecture_census.py` then read **820 over 206**, and removing the test strip
entirely adds only three more. ⇒ **The 89 it was missing were production code,
not fixtures.**

⛔⛤ **AND THAT MAKES FIVE FIGURES, BECAUSE THE HELPER HAD A SECOND DEFECT OF THE
SAME SHAPE.** `da042e39e` (2026-09-18) found it stripped `#[cfg(test)]` items
and not COMMENTS, so `820 / 206` counted prose — three types existed only in doc
comments and doc examples. The reading under the repaired rule was `808 / 203`,
and the same rule reads **806 / 203** today, a genuine fall of two.

⇒ **THE FIX FOR A ROW THAT HAS CARRIED FIVE FIGURES IS NOT A SIXTH, IT IS A DATE
AND A RULE PER FIGURE.** `campaign-metrics.md` now carries that table; this row
owns the argument. ⚠ Note what the two defects had in common: both were the
helper looking at the wrong CORPUS, once too little (the tail cut) and once too
much (prose), and each was invisible to the other's fix.

⚠ **SO BOTH READINGS WERE WRONG, IN DIFFERENT WAYS, AND THE RECONCILIATION
BETWEEN THEM WAS WRONG TOO.** The hand scan's 850 was much closer to the
population than the tool's 726 — its error was the COMPARISON, reporting +16%
growth against a baseline produced by a different rule. The tool's error was the
cut. And the sentence that settled the disagreement — *"those 124 occurrences are
inline test modules, real code but not what this row is about"* — was an
explanation of a gap that was mostly production code.

⇒ **THE RULE THIS ROW ACTUALLY TEACHES: when two honest scans of one tree
disagree, read what each one CUTS before deciding which is right, and never
compare a number against a baseline built by another rule.** Ask the tool, and
then ask the tool what it throws away.

⭐ **AND THE CENSUS CROSS-CHECKS SOMETHING ELSE TONIGHT:** it independently
reports *"explicit process resources with session/generation semantics: 37"*,
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

⭐⛤ **`Q132` IS DECIDED (2026-09-19), AND THE STORAGE HOLD IS DISCHARGED.** It
asked whether a handoff frame holding two session roots should make 185
`Single<.., With<SessionRoot>>` sites run or skip. The ruling answers the
question underneath it instead: **there is exactly one canonical live
`SessionRoot`, and two published/canonical roots are INVALID.** So the frame the
question was about may not exist, the `Single` semantics IS the engine's meaning,
and the scope-aware helpers are for the lifecycle code that legitimately sees
both sides of a handoff. The durable ruling is in
[`maintainer-decisions.md`](../maintainer-decisions.md); moving storage is no
longer held.

⇒ **AND THE RULING REPLACES THE HOLD WITH A HARDER TARGET, WHICH IS THE PART
THAT CHANGES THIS CAMPAIGN'S SHAPE.** C03 is not "move fields to a better owner".
The governing rule is:

> If mutable state can legitimately hold different values for two sessions,
> generations, participants or timelines that could coexist during preparation,
> handoff, rollback, multiplayer or testing, it must carry the appropriate
> explicit scope rather than relying on anonymous App-global singleton identity.

and its converse — App-global mutable state is appropriate ONLY when simultaneous
sessions would legitimately share exactly the same object. ⚠ **The test is
"could two coexisting sessions legitimately differ here?", not "is it a
resource?" and not "does it reset?".** Both of the cheap wins this campaign
advertised were measured away because they asked the second question.

⚠ **EXPLICIT SCOPE DOES NOT MEAN AN ECS CHILD OF `SessionRoot`.** A keyed or
scoped resource, or other clearly owned state, satisfies it. The property is
explicit identity and lifecycle ownership, so this campaign may not use the
ruling to justify reparenting everything.

⚠ **AND THREE POPULATIONS ARE EXPLICITLY NOT IN SCOPE.** Prepared IMMUTABLE data
is generation-scoped and may coexist across generations. Truly
application-global infrastructure — render/device services, logging, asset
infrastructure, networking transport, caches — stays global where that is
genuinely its ownership. User/account settings and durable save data are
SEPARATE AUTHORITIES: they do not become live simulation state by being
App-global, and a mechanical projection from them needs explicit admission into
a session.

⛔ **AND THE RULING NAMES A FAILURE MODE TO GO LOOKING FOR: App-global fallback
state that exists only because session identity was OPTIONAL.** That is the
shape `C04`/`C07` are told to inspect, and it is not to be preserved for the
benefit of old direct-entry tests.

⭐ **THE RULING HAS THREE MECHANICAL HOLDS AS OF 2026-09-19, AND THEY COVER
DIFFERENT FAILURES** — consequence 7 asked for production-composition witnesses
and consequence 8 for candidates not counting as roots:

| hold | what it catches | where |
|---|---|---|
| `the_shipped_app_never_holds_two_session_roots_across_a_handoff` | a second CANONICAL root appearing across a real shell handoff | `game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs` |
| `a_prepared_candidate_never_counts_as_a_canonical_session_root` | a candidate being COUNTED as canonical, and the opposite failure of no candidate being prepared at all | same file |
| `check_session_root_construction_is_declared.py` (maintenance job 45) | a NEW production site minting a root — the one the runtime arms cannot see | `scripts/` |

⚠ **THE THIRD EXISTS BECAUSE THE FIRST TWO DRIVE ONE ROUTE IN ONE APP.** A host
that does not exist yet is invisible to them, and that is how this invariant
will actually break. Four production sites construct a `SessionRoot` today and
each is declared with what it serves; two of the four are demo content plugins
with NO production caller, and one is
`insert_session_world_component`'s fallback, which mints
`active_scope.unwrap_or(SessionScopeId(0))` — an anonymous default identity,
which is the very shape the scoping rule names, with one production caller
(`game/ambition_app/src/app/dev_runtime.rs:626`).

⭐⛤ **THE RULING'S CATEGORIES MAP ONTO THIS CAMPAIGN'S MEASURED 37, AND EVERY
CATEGORY IT NAMES HAS MEMBERS HERE — DERIVED 2026-09-19.** `Q132`'s scoping rule
lists the kinds of state that should generally be scoped. That list is abstract;
`scripts/architecture_census.py` already reports **37 explicit process resources
with session/generation semantics** (`explicit_narrow_lifetime_resources`, two
groups: `SessionOwnedCheckpointState` ×6 and `SessionScopedResources` ×30). Laid
side by side, the ruling stops being a principle and becomes this campaign's
triage order:

| the ruling's category | members among the 37 | n |
|---|---|---:|
| current room / world / session state | `LastCutsceneRoom`, `LastQuestRoom`, `RoomTransitionCooldown`, `MovingPlatformSet`, `SlotInteractionState` | 5 |
| participant state | `ControlledSubject`, `PossessionState` | 2 |
| encounter state | `EncounterRegistry`, `EncounterView`, `BossEncounterRegistry`, `AuthoredOccurrences` | 4 |
| simulation clocks / timeline state | `GameplayElapsed`, `LiveMatchTicks`, `SessionMatchOrdinal`, `ProjectileSeqCounter` | 4 |
| checkpoint / restore state | `SessionCheckpointOperations`, `SessionCheckpointOutcomes`, `AcceptedCheckpointRestore`, `AbandonedCheckpointOperation`, `SessionStartupResume`, `OutstandingCheckpointRequest`, `SaveRestored`, `CustodyBaseline`, `MintedItemBaseline`, `OccurrenceBaseline` | 10 |
| session request / admission queues | `CutsceneTriggerQueue`, `SwitchActivationQueue`, `CutsceneAdvanceRequest`, `PendingLifecycleCommit` | 4 |
| admitted mechanics / configuration | `SessionMechanics`, `BaseGravity` | 2 |
| cutscene / session gameplay state | `ActiveCutscene`, `ActiveConversation`, `CutsceneSkipHold` | 3 |
| transient progression | `QuestRegistry`, `StocksMatchSettled`, `SuddenDeathEntered` | 3 |

⭐ **EVERY ONE OF THE 37 IS ASSIGNED EXACTLY ONCE, CHECKED RATHER THAN EYEBALLED** — the first draft of this table put `OutstandingCheckpointRequest` under queues and `SessionStartupResume` beside the checkpoint group, and both are already MEMBERS of that group of six, so two names appeared twice and the column would not have summed. The assignment is now verified against `architecture_census.py --json` as a partition: no name missing, none repeated.

⚠ **THEY REALLY ARE ANONYMOUS SINGLETONS, SPOT-CHECKED RATHER THAN ASSUMED.**
`GameplayElapsed`
(`crates/ambition_platformer2d_actor_monolith/src/features/mod.rs:238-239`),
`ControlledSubject`
(`crates/ambition_platformer2d_shared_tangle/src/markers.rs:16-17`) and
`LastQuestRoom`
(`ambition_persistence/src/quest/registry.rs:46-47`) are each a plain
`#[derive(Resource)]` newtype with no scope key — the exact shape the rule names,
holding a value two coexisting sessions could legitimately differ on.

⇒ **WHAT THIS DOES AND DOES NOT SETTLE.** It settles the ORDER: the ruling's own
list is the priority, and every category in it is populated here, so no category
is theoretical. It does NOT settle the per-value verdict — the rule's test is
*"could two coexisting sessions legitimately differ here?"*, and for a few of
these the answer may be no. ⚠ `BaseGravity` is the interesting one to decide
early rather than late: it is admitted configuration by the ruling's vocabulary,
it is already a rollback-registered value, and it is one of `Q136`'s live
lost-intent subjects — so it is simultaneously a scoping question and an ingress
question, and moving its ownership before `Q136` rules would prejudge the second.

⚠ AND `SessionMechanics` is `C04`'s subject, held on `Q144`. Leave it there; a
C03 pass that reparents it would take a decision this campaign does not own.

⚠ **AND TWO OF THIS SEQUENCE'S OWN PREMISES WERE MEASURED AWAY ON 2026-09-16**
(see C03's CURRENT STATE): there is no reset-only subset to lift out, and the two
reset lists are a disjoint partition rather than two copies. The owner-by-owner
sequence below is still the right shape; the cheap first win it implies is not
there.

Do not begin by moving all 37 values.
Use a bounded owner-by-owner sequence:

1. Re-run `python3 scripts/architecture_census.py` and confirm the explicit narrower-lifetime list.
2. For each family, state whether the value must exist before `SessionRoot`, only during a live session, or only for presentation.
3. For rollback-registered values, record the current registration and restore boundary before changing storage.
4. Select one coherent family with one owner. `SessionOwnedCheckpointState` is a good first review unit because source already declares its six values as one gameplay-session coordinator. This is a review starting point, not a pre-decided move to one component.
5. Choose storage from semantics: `SessionRoot` component/bundle for live-session state; explicit session-keyed process coordinator when pre-root availability is required; ordinary App resource only when process lifetime is real.
6. Remove the old reset/retirement compensation only after the new owner is the sole authority.
7. Keep direct Bevy query/system-parameter use. Do not add a generic state-container abstraction to hide ECS.

**Exit property:** a future reviewer can name one owner for each migrated fact. Session activation no longer needs to overwrite a process-global copy only to make the next session safe.
