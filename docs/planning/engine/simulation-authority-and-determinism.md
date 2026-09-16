# Simulation authority and determinism

**State:** OPEN. Rollback backend ownership and domain registration are largely
settled. Current work is peer-stable identity, owner-scoped lifetime, deterministic
composition and explicit phase/authority boundaries.

## Goal

A simulation result must be determined by explicit authoritative data and named
composition rules, not by:

- ECS/query iteration order choosing a winner;
- `Local<T>` or process-global memory carrying an authoritative future across a
  rewind;
- host-local lifecycle counters entering peer-stable mechanical identity;
- a rollback-registered component living on an entity that is not on the rollback
  timeline;
- mutable App/editor state being read directly during resimulation;
- two mutable representations independently answering the same mechanical
  question;
- scheduler topology acting as an undocumented conflict-resolution rule.

## Current authority model

An authoritative dynamic fact may need several independent guarantees. Do not
collapse them into one "rollback-safe" label.

### Rewind codec

The domain declares which component/resource state is snapshotted, restored,
entity-remapped and represented in checksum policy. The gameplay domain owns its
rollback declaration beside the types it understands.

The generic runtime collects backend-neutral declarations. The concrete GGRS
backend/session/schedule lives in `ambition_platformer2d_rollback_ggrs`. Do not
recreate a central concrete-type census in the runtime.

### Rollback participation

Registering a component type is not enough. The authoritative entity population
must also participate in the GGRS creation/destruction history. A value can have a
codec and still be absent from the actual rewound population.

### Stable semantic identity

`SimId` is the stable semantic identity used for deterministic simulation objects.
`bevy_ggrs::RollbackId` is frame-history machinery, not gameplay identity.

Use semantic identity when reconstruction, relationships, deterministic selection
or peer comparison need to refer to the same logical object. Do not mint canonical
identity from ECS entity order or an App-local activation count.

Local lifecycle identities such as `SessionScopeId`, `ContentEpoch` and shell/load
correlation ids remain useful for ownership/correlation. They are not substitutes
for peer-stable mechanical identity. The current cross-peer cleanup is tracked by
[ID-PEER in the queue](../queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity).

### Deterministic selection and composition

If multiple valid entities can affect one result, define the rule at the owner
boundary. Use a stable semantic key where selection needs one. When effects do not
commute, define precedence/composition explicitly instead of relying on query
iteration order.

Prefer one shared ordering helper for a domain over several call sites inventing
their own stable order.

### Lifetime ownership

The important lifetime hierarchy is:

```text
process/App
  -> gameplay session
       -> rollback timeline generation
            -> entity / operation
```

`ActiveRollbackAuthority` intentionally owns rollback owner, timeline generation,
contract and health together because they are one authority. A different gameplay
session may not inherit the previous session's rollback health or live mechanical
mirrors. A same-session timeline rebase must not erase evidence of a real desync.

Session-owned values that still live as App resources are candidates for ownership
consolidation, not automatic defects. The architecture census records the current
population and the reset/retirement mechanisms that compensate for App storage.

### Mechanical policy admission

Mutable editor/preferences state is not automatically deterministic input. Live
mechanical editor domains use a proposal -> admission -> publication boundary so
a rollback host can rebase/refuse before simulation sees the new authority.

Direct `UserSettings` reads have been removed from the simulation schedule. Values
that affect mechanics are projected into admitted policy, such as per-seat control
frame modes and `PlayerDamagePolicy`. The remaining damage-policy lifetime is
[Q127](../awaiting-maintainer-decision.md#q127--are-difficulty-assist-and-player-damage-modifiers-match-wide-or-participant-specific).

## Established architecture

These decisions are current foundations; do not reopen them without new evidence.

### Domain-owned rollback declarations

Each gameplay domain owns the registration of its rollback types. The host/backend
composes offers; it does not enumerate leaf gameplay types itself.

`central-rollback-does-not-enumerate-domains` in
`scripts/check_absence_contracts.py` protects this boundary.

### Wire-format changes are explicit

Rollback schema names and encoded shapes are compatibility data. A source-file
move is not a reason to rename a stable schema entry. A real shape/name change must
update the declared wire-format baseline intentionally.

Use the repository's rollback baseline/checks; do not copy their counts into this
page.

⛔⛤ **AND THE FINGERPRINT HASHES PROSE, MEASURED 2026-09-16.** `schema_dump()`
emits four columns — name, kind, wire type, and a human-readable `detail` — and
`compute_schema_fingerprint` hashes the whole dump. So the `detail` string is
inside content identity. Probed by changing TWO string literals in
`rollback_component_clone` / `rollback_resource_clone` and changing nothing else:
`the_rollback_schema_matches_its_recorded_baseline` fails and reports it in these
words — *"This is a WIRE-FORMAT change: the fingerprint is part of content
identity, and two peers whose schemas differ cannot agree about a snapshot."*

⇒ **CORRECTING A MISLEADING DESCRIPTION IS INDISTINGUISHABLE FROM CHANGING AN
ENCODING**, to the guard and to every peer. That is a disincentive pointed at
exactly the repair S7 needs: the 99 rows whose `detail` asserted coverage the kind
cannot establish would cost a `GGRS_ROLLBACK_SCHEMA_VERSION` bump whose log entry
would be the only non-mechanical one in that log.

✔ **PAID, 2026-09-16 — v194.** The repair landed and the prediction held exactly:
the bump is real, its log entry opens *"nothing mechanical changed, and that is
the point of this entry"*, and it is the only such entry in the log. ⓘ One
correction to the shape of the tax, learned by paying it: it is per EDIT, not per
row. One bump covered all 99. That is why this defect is survivable, and why it
goes unnoticed.

⇒ The one-fact-one-owner reading is that the mechanical schema is
`name | kind | wire type` and `detail` is DOCUMENTATION of it: keep `detail` in
the readable baseline, exclude it from the fingerprint, and descriptions become
correctable while the guard still catches every real wire change. ⚠ That is
itself a one-time fingerprint change over a mechanical identity with an absence
contract on it (`rollback-wire-format-changes-are-declared`), so it is a decision
rather than a cleanup — recorded here, not taken.

### Explicit simulation phases

The GGRS simulation schedule has named Ambition phases and currently uses a
single-threaded executor. This is a measured implementation choice, not a
requirement that deterministic simulation must always be single-threaded.

Ordering edges are meaningful only when the referenced set has real members in
the composition. `scripts/check_set_pins_have_engine_members.py` detects engine
pins to empty engine sets.

### Session-owned rollback authority

Rollback contract/health is scoped to the gameplay session. Session activation and
retirement establish/clear the session mirrors together rather than relying on
ad-hoc cleanup in each consumer.

Do not replace owner-scoped lifetime with a growing list of "clear on quit"
systems.

## Current work

### S5 — phase and ownership decomposition

Split high-authority systems only when the split creates a real semantic owner or
phase. Useful evidence includes:

- independent authoritative domains being read/written;
- correctness-sensitive ordering;
- broad queries that cross ownership boundaries;
- rollback participation differences;
- mutation mixed into proposal/decision logic;
- duplicate derivations of one authority.

A `SystemParam` or `QueryData` is useful when it names one concept. Hiding many
unrelated resources behind one parameter is not decomposition.

The current A4 control/body packet is one concrete customer; use the actual Bevy
schedule realization rather than retired abstract set names. See
[A4 in the queue](../queue.md#a4--separate-control-authority-from-body-execution-on-the-real-schedule).

### S6 — move session-owned state toward session ownership

The census identifies App-stored resources whose semantic lifetime is a gameplay
session or content generation. Existing reset systems and owner stamps often make
that correct today, but each reset-only mirror is a candidate for a stronger
storage owner.

Do this only after A10/peer identity establish the live/candidate session shape.
For every candidate value:

1. name its semantic lifetime;
2. name whether it must exist before `SessionRoot` construction;
3. preserve rollback registration/restore semantics;
4. choose `SessionRoot` component ownership, a session-keyed coordinator, or a
   genuine App resource based on those facts;
5. remove reset/synchronization machinery only after the new owner makes it
   redundant.

Do not move state merely to reduce a resource count.

### S7 — canonical checksum coverage

Rollback registration and session checksum coverage are different properties.
The rollback baseline contains rows intentionally cloned/restored for localization
that are not themselves encoded into the session checksum. Some are legitimate
because another canonical projection represents the same fact; some may be dead
or diagnostic-only state; a mechanically relevant value with no peer-comparison
projection needs review.

Do not preserve a copied count of these rows here. Recompute the population from
`game/ambition_app/tests/rollback_schema_baseline.txt`, then classify each row by:

1. production readers;
2. whether it can affect future mechanical behavior;
3. which canonical projection/checksum represents it, if any;
4. whether the state is reachable in production.

A green float-finiteness or codec test does not prove that every restored value is
peer-compared.

**THE POPULATION, COMPUTED AT SCHEMA 193 (2026-09-16).** 494 rows, 443 distinct
types. Asking `RollbackEntryKind` rather than matching kind strings — the enum
owns `carries_state()` and `feeds_peer_checksum()`, and a hand-kept list of kind
names has already hidden 25 of 29 registrations once:

- **175 types CARRY state and are never peer-compared.** That is the S7
  population. Recompute it, do not copy this number.
- Zero types are registered both cloned and checksummed, so no type is covered by
  a sibling registration of ITSELF. Every claim of coverage is a claim about a
  DIFFERENT type.

⛔⛤ **AND THE REASON COLUMN IS NOT AUTHORED PER ROW — IT IS EMITTED BY THE
REGISTRAR METHOD, WHICH MAKES THIS ONE CLAIM ASSERTED ABOUT 99 TYPES THAT WERE
NEVER INDIVIDUALLY EXAMINED.** 99 rows read *"state checksum supplied by another
authoritative projection"* (removed at schema v194; quoted here as the defect this
section recorded), and that string was a literal inside
`rollback_component_clone` and `rollback_resource_clone`, whose only bound is
`T: Clone`. Nobody wrote it 99 times; nobody wrote it once per type either.
⚠ It was spelled TWICE MORE than that — once per registrar, in two crates, with
nothing comparing the copies — until `879a5a1a3` collapsed all 15 schema
sentences onto `runtime::rollback::detail`. The dump is byte-identical, so that
commit is a pure ownership move; it is what makes the sentence a one-place edit.

⇒ **THE DEFECT IS THAT THE KIND ASSERTS SOMETHING ONLY THE TYPE CAN KNOW.**
Whether another authoritative projection covers a fact is a property of the
value, not of the snapshot strategy chosen for it — so `rollback_component_clone`
is claiming coverage it has no way to establish, on behalf of every caller.

⛔⛤ **AND THE SENTENCE DECIDED WHO GOT MEASURED. THE 99 WERE NEVER IN THE CENSUS
THAT EXISTS TO FIND THEM.** `scripts/measure_unchecksummed_rollback_rows.py` is
titled *"rollback rows that are SNAPSHOTTED but contribute nothing to the session
checksum"* — which is `feeds_peer_checksum() == false` for a value-bearing kind,
175 rows. It selected on `"not in the session checksum" in detail`: **59 of 175**.
The instrument named the concept in its title and matched the SPELLING in its
filter, and the 116 it missed were missed because they carry the reassuring
sentence. Same mechanics, opposite attention.

⇒ Fixed: the census selects on `UNHASHED_KINDS = ("component-clone",
"resource-clone")` and prints the population split by whether the row carries a
desync-localization probe — a mechanical fact, unlike the coverage claim it
replaced:

| rows | self-description | probe |
|---|---|---|
| 99 | *not in the session checksum* — was *state checksum supplied by another authoritative projection* until v194 | **none** |
| 59 | *value-probed for localization, not in the session checksum* | yes |
| 17 | entity handle / SET / keyed MAP remapped, probed through stable sim identity | yes |

⇒ **99 of 175 unhashed rows have NEITHER a checksum contribution NOR a probe.** A
probe does not make a row peer-compared; it tells a desync hunt where to look.
These 99 offer neither, and that is a checkable statement where the old one was
not. The float-bearing count over the true population is **79 of 175**.

⛔⛤ **AND THE GUARD THAT CAUGHT THE SPELLING BUG WENT BLIND THE SAME DAY, TO A
CHANGE THAT HAD NOTHING TO DO WITH IT.** The first fix floored the census at 120
rows, on the reasoning that a revert to prose-matching would return 59 and trip
it. Then v194 reworded the 99 to *"not in the session checksum"* — the words the
old selector matched — so a reverted selector now returns **158**, sails over the
floor, and reports a population missing 17 rows. Poisoned and confirmed: with the
prose filter restored, the count floor passes and only the member-diff arm fails.
⇒ The surviving arm recomputes the population from the baseline by a DIFFERENT
expression and diffs the MEMBERS. A count floor is a claim about a number; the
thing it was protecting was a list.

⚠ **AND WIDENING IT FOUND TWO ROWS THE INSTRUMENT IS STRUCTURALLY BLIND TO.**
`entity.name` and `entity.transform` are `bevy::prelude::Name` and `Transform` —
defined outside this repository, so the `git grep '(struct|enum) X'` index cannot
see them and reported both as having no float-bearing fields. `Transform` is three
`Vec3`/`Quat`. They are named in `EXTERNAL_TYPES` now with their floats stated.
ⓘ Their registration had no recorded reason: the comment above them described
portal-gun timers that had left for `ambition_portal2d::register_rollback_state`,
and the carve moved the code while leaving the reason to be read as theirs.

⇒ Two ways out, and the first is cheaper than sweeping 175 rows: either the
registrar TAKES the coverer (the projection or type that does compare this fact)
as an argument, so a caller must name it or say there is none; or the string
stops claiming coverage and says what the kind actually knows — that this value
is snapshotted and not compared. The second is a one-line change that makes 99
rows stop asserting something unchecked; the first is the version that could go
stale visibly.

Two worked examples, to show the classification is not uniform:

- ✔ `Dormant` (`actor.dormant`) — the claim HOLDS, and the registration is right
  too. Its own doc says *"derived every tick; never authored, never persisted"*,
  and `assess_dormancy` recomputes it from observer `BodyKinematics`, which are
  canonical. ⛔ **THAT SENTENCE IS NOT A DEMOTION CANDIDATE, AND READING IT AS
  ONE IS THE TRAP.** `Without<Dormant>` filters three production queries in
  `features/ecs/actors/update.rs`, so a rewind that dropped the marker would put
  a sleeping body back into the decision phase for one advance. The registration
  beside its sibling `SensesUndecided` says so in the repository's own words:
  *"'Re-derived next tick' is not a reason to omit it: `ITEM 0` of this project's
  own record is a component declared derived, dropped by a restore, and read
  before its writer ran again. Presence is authoritative because a query FILTERS
  on it."*

  ⇒ **A COMPONENT WHOSE PRESENCE IS READ BY A QUERY FILTER IS AUTHORITATIVE EVEN
  WHEN ITS VALUE IS DERIVED.** "Derived" describes how it is COMPUTED; rollback
  cares about whether anything READS it before its writer runs again. A doc
  sentence that sounds like a demotion candidate is usually answering the first
  question. A keyword sweep of the 212 types registered through a `*_clone`
  method whose declaration doc matches `derived|recomputed|never authored|never
  persisted` returns 21 CANDIDATES — and most of those say "derived" about
  something else entirely (`ActorRenderSize`'s collision box, `CapturedBy`'s
  inverse, `AuthoredHurtboxes`' absence). The demotion population looks close to
  zero; 21 is a floor of candidates, not a count of defects.
- ❓ `PlayerSlot` (`actor.player_slot`) — the claim is UNVERIFIED and I could not
  confirm it. It carries a `u8`, its own doc calls it *"the canonical 'which
  player?' handle"*, it appears exactly once in the baseline as `component-clone`,
  and no canonical projection naming the per-body slot was found. This is
  recorded as a QUESTION, not a defect: seating is peer-agreed through
  `ActiveMatch::peer_stable_checksum`, which hashes the seat COUNT, and whether
  that constrains per-body slot assignment has not been measured.

⇒ The next step is not a sweep of 175, and it is not a demotion pass either —
that population looks close to zero, measured above. It is to stop the KIND
asserting coverage it cannot know, then classify the payloads that can change
mechanical behaviour.

⚠ **A METHOD NOTE, BECAUSE THIS PAGE IS WHERE SOMEBODY WILL REPEAT IT — AND IT
WAS WRONG TWICE.** The first version of this section said "93 unverified claims"
and inferred, from the repeated string in the artifact, that somebody had written
the sentence 93 times. Both halves failed:

1. **The authorship was invented.** The generator is one literal in two
   registrar methods. Reading a repeated value out of a derived artifact tells
   you about the artifact, not about how it came to be — open the emitter before
   describing the population's authorship.
2. **The count was an instrument artifact, and the artifact never held it.** The
   baseline carries **99** rows (94 `component-clone`, 5 `resource-clone`), and
   `git show <sha>:game/ambition_app/tests/rollback_schema_baseline.txt` says 99
   at every recent commit that touched the file — so 93 was never a reading of
   anything. Which step of the ad-hoc script dropped six is not recoverable from
   the artifact, and guessing a mechanism would repeat the first error in a new
   costume.

⇒ The act-trigger: a count taken by a script written for one question must print
its **good value** — here, the baseline has 494 rows and 99 is a subset of them,
so `grep -c` against the committed file is the check, and it takes one command.
A peer ran it and corrected the number; nothing in my own output would have.

### Host-to-host determinism witness

An older two-host measurement found a duel that agreed for hundreds of ticks and
then diverged while each host remained repeatable on its own. That evidence was
stamped to an older commit and does **not** establish that current HEAD still
diverges.

Before drawing architecture conclusions, reproduce on current HEAD on both hosts
and capture the **first divergent authoritative state**, keyed by stable identity.
Do not reason backward from final bout statistics. If the current build no longer
reproduces, record closure rather than preserving the old investigation as a
current defect.

## Extension-state coverage

Dynamically declared extension state uses the same rules. Registry metadata alone
is not rollback participation. Module scratch heaps, statics, RNG cursors and
coroutine state may not remember authoritative futures outside the declared
state/execution contract.

Use [`extension-state-and-execution.md`](extension-state-and-execution.md) and
[`extension-domain-contracts.md`](extension-domain-contracts.md) for extension
read/write/request semantics. Extensions do not get a second rollback backend or a
weaker identity/lifetime model.

## State projection rule

Read models are allowed and encouraged. They must be one-way projections from
one authority.

If a projected component/resource contains fields that another system mutates as
independent authority, either split the representation or move that authority.
Do not rebuild a projection by saving and restoring selected fields around the
rebuild.

## Test and measurement topology

Use the smallest host that still contains the property being asserted.

| Property | Required proof shape |
|---|---|
| deterministic simulation | real headless `GgrsSchedule` / sync-test session |
| runtime-created rollback population | scenario that creates the population before rewind |
| cross-session isolation | shell/app host that creates, retires and creates real sessions |
| editor/mechanical admission | production ordering before rollback advance plus ownership arms |
| physical input/rebinding | real input/session host |
| rendered/raster behavior | rendered hardware measurement |
| durable persistence | fresh-process reconstruction |
| capability composition | explicit supported profile/feature matrix |
| peer-stable identity | two Apps/peers with intentionally different local history |

A helper that manually installs the resource/system under test cannot prove that
the production composition installs or orders it correctly.

## Static guards

These guards are discovery/structural checks, not substitutes for runtime proof:

- `scripts/check_rollback_mutators_run_in_sim.py` — catches known rollback-state
  mutation registered outside the rewinding schedule;
- `scripts/check_set_pins_have_engine_members.py` — catches engine ordering pins
  to empty engine sets;
- `scripts/check_absence_contracts.py` — includes the domain-ownership and wire
  compatibility absence contracts.

When a static census has an unattributed/unknown bucket, it can establish a floor,
not prove zero.

## Acceptance

- adding a rewindable domain type changes its domain declaration, not a central
  gameplay-type census;
- representative dynamic authoritative families survive rewind/recreation with
  correct semantic identity;
- no known future-affecting state depends on non-rewinding scratch/process memory;
- deterministic selection does not depend on raw ECS iteration order;
- host-local lifetime counters do not determine peer-stable mechanical identity;
- one gameplay session cannot consume another session's rollback health or live
  mirrors;
- same-session rebases preserve real desync evidence;
- simulation systems have named authority/phase contracts rather than parameter
  packing disguising unrelated ownership;
- restored mechanically relevant values have an explicit peer-comparison story.

## Non-goals

- another custom rollback/snapshot engine;
- pushing `bevy_ggrs` into every leaf domain;
- a universal raw-spawn wrapper;
- exhaustive pairwise `.before()`/`.after()` edges instead of semantic phases;
- scheduler parallelism as an architecture objective without new measurement;
- moving state merely to reduce crate/resource/system counts;
- source-text policy where a type/API/runtime witness can make the invariant
  structural.
