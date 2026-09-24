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

⛔⛤ **THAT SENTENCE HAS A WITNESS AS OF 2026-09-16, AND IT WAS VIOLATED IN
PRODUCTION UNTIL THEN.** `collect_perception_peers` fell back to
`format!("e{}", entity.index())`, and that string is the KEY of a `BTreeMap`
inside `PerceptionMemory`, registered `rollback_component_canonical` — so an ECS
allocation order was inside a peer checksum. The same shipped route reached first
in one host and third in another perceived the player as `e888` and `e1026`. ⚠
The checksum was the smaller half: that map's iteration order also breaks the tie
between two equally-confident hostiles, so the two peers' NPCs would chase
different targets. Held by
`the_peer_visible_surface_does_not_record_which_route_the_host_visited_first`
(`game/ambition_app/tests/shell_host_lifecycle.rs`), which is the arm to extend
when this rule needs a new subject — a type census cannot see this class, because
the offending value is a `String` field of a legitimately-registered component.

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
frame modes and `PlayerDamagePolicy`. The remaining damage-policy lifetime was `Q127`, and it is
[RULED](../maintainer-decisions.md), 2026-09-19: there is to be no generic
one-dimensional engine difficulty architecture. Difficulty is game policy
expressed as presets; participant handicaps and CPU brain levels are separate
concepts, and participant-specific assist/handicap state stays distinct from
game/match policy. ⚠ The topic is DEPRIORITISED on purpose — preserve enough
architecture not to be boxed in later and get the default game playing
exceptionally well first.

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

⇒ The one-fact-one-owner reading looks like this: the mechanical schema is
`name | kind | wire type` and `detail` is DOCUMENTATION of it — keep `detail` in
the readable baseline, exclude it from the fingerprint, and descriptions become
correctable while the guard still catches every real wire change. ⚠ That is
itself a one-time fingerprint change over a mechanical identity with an absence
contract on it (`rollback-wire-format-changes-are-declared`), so it is a decision
rather than a cleanup — recorded here, not taken.

⛔⛤ **AND THAT READING IS REFUTED BY MEASUREMENT ON THE PAGE IT ROUTES TO.**
`Q122` counted the 493 rows by kind: **225 are uniform** (one sentence per kind,
derivable from the `kind` column beside it) and **268 VARY**, carrying facts
`kind` does not encode — `component-clone` alone distinguishes *entity handle
remapped* from *entity SET remapped* from *keyed entity MAP remapped*, and 22
`resource-clone-custom-checksum` rows each name what their `fn(&T) -> u64`
actually covers. Excluding `detail` wholesale would stop the fingerprint seeing
an entity-remapping change. ⇒ `detail` is not documentation OF the schema; for
half the rows it IS the schema.

✔ **THE RULE THAT FITS BOTH HALVES IS IMPLEMENTED AND GREEN, at the granularity
the dump already has** — landed 2026-09-16 for the peer-checksum slice of the
tooling ratchet: **`detail` is kept exactly where it DISTINGUISHES rows of the
same kind, and dropped where it does not.** A kind whose rows all carry one
sentence has a `detail` the `kind` column already implies; a kind whose rows
differ is using it to say something `kind` cannot. Of the 144 checksum-feeding
rows, 48 carry theirs. The control and the positive differ only in their subject:
rewording the 7 uniform `component-clone-cursor` rows stays green, rewording one
of the 22 varying ones reddens.

⚠ That is the tooling ratchet, NOT `compute_schema_fingerprint`, which still
hashes the whole dump. So the repository now answers this question two opposite
ways in two places — which is a second witness for `Q122`'s ruling and evidence
that the split is implementable, not the ruling itself.

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

⭐ **RECOMPUTED 2026-09-17 AT SCHEMA 198, because this section says to.**
`python3 scripts/measure_unchecksummed_rollback_rows.py` reports **177 rows
outside the session checksum, 101 of them with no value projection** (78 read by
an unfiltered per-tick query, 21 read only as PRESENCE, 1 gated or point reads).
⚠ The `99` this section discusses below is that same no-projection figure at
schema 193: **`Q142` added `ReleaseOnDeath` and `RecharacterizeBody` as
`component-clone` markers on 2026-09-17, which is the whole of the +2.** ⇒ And
the control that makes the +2 readable rather than alarming: **NO PROJECTION ×
READ EVERY TICK × FLOAT-BEARING is still 25**, because both new rows are ZSTs.
A population growing while its float-bearing subset holds still is the shape a
reader should check for before treating a moved total as a finding.
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
| 101 | *not in the session checksum* — was *state checksum supplied by another authoritative projection* until v194 | **none** |
| 59 | *value-probed for localization, not in the session checksum* | yes |
| 17 | entity handle / SET / keyed MAP remapped, probed through stable sim identity | yes |

⇒ **101 of 177 unhashed rows have NEITHER a checksum contribution NOR a probe.**
A probe does not make a row peer-compared; it tells a desync hunt where to look.
These 101 offer neither, and that is a checkable statement where the old one was
not. The float-bearing count over the true population is **78 of 177**.

⭐ **RE-DERIVED 2026-09-18 — the table above read 99 / 175 / 79 and the drift is
two new rows, both landing in the no-projection column.** Ask
`python3 scripts/measure_unchecksummed_rollback_rows.py`, which prints every
line of this section; this page owns the ARGUMENT and the script owns the
cardinalities. ⚠ The three downstream owners already defer here by name —
`netcode.md` says *"the count of what has been MEASURED lives in S7 and in the
ID-PEER row, not here"* — so this is the only copy to refresh.

⛔ **AND TWO DIFFERENT NUMBERS IN THIS SECTION ARE BOTH 78 TODAY, WHICH IS A
COINCIDENCE AND NOT AN IDENTITY.** 78 rows are float-bearing across the whole
177; 78 of the 101 no-projection rows are read by an unfiltered per-tick query.
They were 79 and 78 yesterday and will part again. Read the sentence, not the
figure.

⭐⭐ **AND THE CLASSIFICATION S7 ASKED FOR IS NOW MEASURED, NOT PLANNED.** The
next step this section named was *"classify the payloads that can change
mechanical behaviour"*. The census already held a reader triage — and had never
called it. `reader_sites` was exercised only by its own test, so a capability with
one customer in a test file looked exactly like an absent one. Wired into the
report and run over the 99 rows whose `detail` names no value projection:

| rows | how it is read | what a divergence does |
|---|---|---|
| 78 | an unfiltered per-tick query, or a `Res`/`ResMut` parameter | propagates on the next frame |
| 19 | PRESENCE only, through a query filter | the component's existence is authoritative even where its value is derived |
| 1 | gated or point reads only | a human must decide when it is read |
| **1** | **nothing in production** | — |

⇒ **78 of 99 are read every tick and are not in the checksum**, and the only
thing that would localize a divergence in one is a carrier COUNT.

⛔⛤ **A CORRECTION TO THIS SECTION, MADE THE SAME DAY IT WAS WRITTEN, AND IT IS
THE SECTION'S OWN LESSON ONE LEVEL DOWN.** This first read *"78 of 99 have no
probe"*. They all have one. `rollback_component_clone` installs
`ChecksumProbe::presence_for::<T>` — a carrier count — eleven lines below the
`detail` string it records, and I classified from the string. Having just spent
two commits on a census that matched a SENTENCE instead of the property in its
own title, I then read a sentence for a fact the code owns.

⇒ **THE AUTHORITY FOR PROBE STRENGTH IS `RollbackChecksumProbes`, AT RUNTIME, AND
IT ALREADY EXISTED.** `ProbeStrength` is `Value` / `Complete` (zero-sized, where
presence IS the value, decided by `size_of::<T>() == 0`) / `Presence`, and
`presence_only_type_names()` enumerates the weak half.
`every_presence_only_probe_is_named_with_its_reason`
(`game/ambition_app/tests/rollback_exit_oracle.rs`) already asserts every one is
listed with a real reason. Measured 2026-09-16 from that test's own output:
**364 probes — 220 value, 28 complete, 116 presence-only, 45 of those derived.**
A static scan of the text baseline cannot answer this and the census now says so
instead of implying an answer.

⛔⛤ **AND THE TRIAGE'S FIRST NUMBER WAS 20 UNREAD, NOT 1 — THREE MORE BLIND SPOTS,
ALL IN THE REASSURING DIRECTION.** It looks for a BORROW (`&T`), and two whole
classes of read are not borrows:

- **A marker is never borrowed, only filtered on.** `FeatureSimEntity` reported
  NO PRODUCTION READER with **81** `With`/`Without`/`Has` sites. Every marker
  component in the population read as unread.
- **A resource is read through `Res<T>`.** `resource-clone` rows were outside this
  census entirely until the selector moved to the kind, so nothing here had ever
  been a resource — `SaveRestored`, `FriendlyFire` and `PortalFrameHistory` each
  reported zero while each is a live `Res`/`ResMut` parameter.
- **The scan root read `crates/` and `game/`** while the workspace has 51 more
  `.rs` files under `tests/`, `fixtures/` and `examples/`. Widening to the two
  non-test roots moved no row HERE, which is evidence the blind spot was empty for
  this population, not that it cannot matter.

⇒ Both patterns are pinned by known-answer controls (`FeatureSimEntity` must be
seen as presence-filtered, `SaveRestored` as a resource read) with the inverse
arms as anti-looseness controls, and both were poisoned.

✅ **`player.local_marker`, THE ONE ROW WITH NO PRODUCTION READER, IS DELETED** (schema
v214): `LocalPlayer` was a per-peer marker in snapshotted state that nothing read.

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
⛔⛤ **AND THEIR REASONS ARE RECORDED IN ANOTHER CRATE'S TEST, WHICH IS THE
FINDING.** I first wrote that these two registrations had no recorded reason.
They have one: `rollback_exit_oracle.rs`'s `PRESENCE_ONLY` allowlist says `Name`
is *"authored debug name; immutable at runtime"* and `Transform` is *"presentation
transform, republished from BodyKinematics every frame"*, and that allowlist is
what makes them DECIDED placements rather than accidents. A reader standing at the
registration site cannot see it — what sat there instead was a stranded comment
about portal-gun timers that had left for
`ambition_portal2d::register_rollback_state`, which the carve left behind to be
read as theirs.

⇒ A registration's justification living in a test's allowlist in a different crate
is the same shape as the `detail` column's problem one level up: the fact has an
owner, and it is not where the fact is used. Both are now cross-referenced; the
one-owner question is `Q122`'s.

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

  ⭐⭐ **AND THE INVERSE READING NOW HAS AN INSTRUMENT, WHICH THIS PARAGRAPH DID
  NOT UNTIL 2026-09-17.** Everything above asks whether a REGISTERED row should
  be demoted. The opposite question — a presence-filtered component with no row
  at all — is invisible to the schema baseline, which member-diffs the rows that
  exist. `scripts/check_presence_filtered_state_is_rollback_registered.py`
  measures it: **95 components are defined in a registering crate and read
  through `With`/`Without`/`Has` outside test code; 73 are registered, 18 are
  waived by name with the measurement beside each, and 4 are neither.** All four
  are one-shot latches whose CONSUMPTION is the state a rewind would have to put
  back, and they are filed as `Q142`. ⛔ Its own first version could not see
  `Dormant` — the component this bullet is about — because all three filter sites
  spell it with a module path; the poison that caught that is planted as a test.
- ❓ `PlayerSlot` (`actor.player_slot`) — the claim is UNVERIFIED and I could not
  confirm it. It carries a `u8`, its own doc calls it *"the canonical 'which
  player?' handle"*, it appears exactly once in the baseline as `component-clone`,
  and no canonical projection naming the per-body slot was found. This is
  recorded as a QUESTION, not a defect: seating is peer-agreed through
  `ActiveMatch::peer_stable_checksum`, which hashes the seat COUNT, and whether
  that constrains per-body slot assignment has not been measured.

⇒ **BOTH HALVES OF WHAT THIS PARAGRAPH ONCE CALLED "NEXT" ARE DONE.** It is not a
sweep of 175 and not a demotion pass — that population looks close to zero,
measured above. It was to stop the KIND asserting coverage it cannot know, which
landed at schema **v194** (the `detail` column now says *"bevy_ggrs clone
snapshot; not in the session checksum"* and claims nothing about a coverer), and
then to classify the payloads that can change mechanical behaviour, which is the
triage table above and the 25-row intersection below. What remains after those is
a DECISION, not a measurement: `Q122` on where a registration's justification
lives, and the 25.

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

⛔⛤ **AND THE 78 NARROW TO 25 WHEN THE THREE CONDITIONS ARE INTERSECTED.** "Read
every tick" and "outside the checksum" are two thirds of a desync; the third is a
value that can drift without anybody making a mistake. Measured 2026-09-16 over
the no-value-projection rows:

> **no value projection × read by an unfiltered per-tick query × carries a
> float-bearing field = 25 rows.**

`actor.animation_facts`, `actor.interaction`, `actor.render_size`,
`actor.spawn_baseline`, `actor.sprite_offset`, `actor.sprite_posed_body`,
`boss.capability`, `boss.config`, `boss.death_animation`, `boss.overrides`,
`combat.tuning`, `encounter.camera_zoom`, `entity.transform`, `feature.hazard`,
`gravity.flip_switch`, `item.ground_item`, `lifecycle.room_visual`,
`mount.authored_size`, `mount.mass`, `mount.mountable`,
`player.blink_camera_state`, `portal.emission`, `portal.gun_pickup`,
`portal.placed`, `portal.shot`.

⇒ A float drifts by rounding rather than by a logic error, the drift compounds,
the reader sees it on the next frame, and no checksum two peers compare can see
any of it. That is the whole shape in one row, twenty-five times.

⚠ **THIS IS A RANKING, NOT A DEFECT LIST, AND THE DISCRIMINATOR IS WHETHER
ANYTHING MUTATES THE VALUE AFTER SPAWN.** A value nobody writes cannot diverge
between two peers who authored it from the same content, so the 25 split on that
question and not on how alarming the name sounds. Measured 2026-09-16 by
`git grep -E '(&mut +T\b|ResMut<T>|Mut<T>)'` over `crates/`, `game/` and
`examples/`, excluding tests — **12 of the 25 are mutably borrowed in
production**:

| row | mut sites | row | mut sites |
|---|---|---|---|
| `item.ground_item` | 7 | `gravity.flip_switch` | 1 |
| `actor.animation_facts` | 6 | `player.blink_camera_state` | 1 |
| `portal.placed` | 4 | `portal.emission` | 1 |
| `boss.death_animation` | 2 | `portal.gun_pickup` | 1 |
| `actor.render_size` | 1 | `portal.shot` | 1 |
| `feature.hazard` | 1 | `entity.transform` | republished every frame |

⭐ **RE-DERIVED 2026-09-19 AFTER `Q137`: THE POPULATION IS 24 AND THE MUTABLY
BORROWED ARE 11.** `scripts/measure_unchecksummed_rollback_rows.py` — the same
instrument, re-run rather than re-read — now prints 24 rows, and the one
missing is `gravity.flip_switch`, whose whole vertical was deleted (see below).
It was one of the twelve, so the mut-borrow column loses exactly its row and
nothing else moves. ⚠ THE TABLE AND THE LIST ABOVE ARE LEFT AS TAKEN on
2026-09-16, with this line as the correction: restating a measurement's numbers
in place, without re-running its method, is how a census comes to describe a
tree nobody measured.

The other 13 — `actor.interaction`, `actor.spawn_baseline`,
`actor.sprite_offset`, `actor.sprite_posed_body`, `boss.capability`,
`boss.config`, `boss.overrides`, `combat.tuning`, `encounter.camera_zoom`,
`lifecycle.room_visual`, `mount.authored_size`, `mount.mass`, `mount.mountable` —
have no mutable borrow anywhere in production. For those, a float field is not a
drift risk; it is authored data that two peers read from the same content, and
being outside the checksum costs nothing.

⛔⛤ **MY FIRST VERSION OF THIS PARAGRAPH NAMED FIVE ROWS FROM INTUITION AND WAS
WRONG ON TWO, IN BOTH DIRECTIONS.** It called `actor.spawn_baseline` and
`mount.mass` sim-written — both have **zero** mutable borrows — while missing
`item.ground_item` (7) and `actor.animation_facts` (6), the two largest writers in
the whole set. The names that sound like state (`spawn_baseline`, `mass`) are
authored constants and the ones that sound like inventory bookkeeping are the
mutated ones. A row's NAME is not a reading of its write set.

⭐⭐ **AND THE TWELVE ARE NOW MEASURED ACROSS TWO LOCAL HISTORIES AND FIVE
ROOMS, 2026-09-17 — ELEVEN OF THEM WITH CARRIERS, ALL ELEVEN AGREEING.**
`two_local_histories_agree_about_the_sharp_unchecksummed_rows`
(`game/ambition_app/tests/shell_host_lifecycle.rs`) is the complement of the
peer-visible census arm: it asserts each of these twelve rows does NOT feed the
peer checksum — so the two arms cannot drift into reading one surface while
claiming to split it — and then compares the probe census of exactly these rows
between a host that reached the shipped Ambition route first and one that reached
it third. **12 of 12 registered; `actor.animation_facts`, `actor.render_size`,
`boss.death_animation`, `entity.transform`, `feature.hazard`, `item.ground_item`,
`player.blink_camera_state`, `portal.emission`, `portal.gun_pickup`,
`portal.placed` and `portal.shot` carried state and agree at EVERY ONE of the
121 observation points — step 0 and after each of ticks 0..119 — in every room
the arm walks.** That eleven is pinned by SET EQUALITY, not by a floor, so a row
losing its carriers is a red rather than a quieter green. The arm prints the
per-room split, because a union cannot say which walk carries a row:

| room | sharp rows it carries |
|---|---|
| `<authored>` (what pressing launch reaches) | `actor.animation_facts`, `actor.render_size`, `entity.transform`, `item.ground_item`, `player.blink_camera_state`, `portal.gun_pickup` |
| `portal_lab` | `actor.animation_facts`, `actor.render_size`, `entity.transform`, `player.blink_camera_state`, `portal.emission`, `portal.placed` |
| `basement_hazards` | `actor.animation_facts`, `actor.render_size`, `entity.transform`, `feature.hazard`, `player.blink_camera_state` |
| `portal_bridge` (driven, with the gun) | `actor.animation_facts`, `actor.render_size`, `entity.transform`, `player.blink_camera_state`, `portal.gun_pickup`, `portal.placed`, `portal.shot` |
| `basement_boss` | `actor.animation_facts`, `actor.render_size`, `boss.death_animation`, `entity.transform`, `player.blink_camera_state` |

⇒ `item.ground_item`, `portal.emission` and `boss.death_animation` have exactly
one carrier room each, so any of those three rooms leaving this walk costs a row
— which is what the set equality is there to say out loud.

⛔⛤ **A ROOM IS THE POPULATION, AND UNTIL 2026-09-17 THIS RANKING HAD ONE OF
THEM.** The arm walked the authored start room alone, six rows carried nothing,
and this page read that as *"a fact about the ROUTE and not about the rows"*. For
two of the six it was a fact about the ROOM: `portal_lab` authors fourteen
`Portal` placements and `basement_hazards` three `DamageVolume`s. Pinning the
start room with `StartRoomOverride` — carrying `StartRoomMustResolve`, because an
override that does not resolve boots the authored room and says so only in a log
line — moved the measured coverage from six to eight with no new fixture, no
input road, and no change to the simulation. ⭐⭐ **AND THE NINTH CAME FROM THE
ONE THING A ROOM CANNOT AUTHOR: A PRESS.** `portal.shot` exists because somebody
fired, so the fourth walk starts in `portal_bridge` — the player at x=94, the
authored `PortalGunSpawn` at x=180 with a 20px half-extent — and drives <!-- cite-ok: `PortalGunSpawn` is an authored LDtk entity identifier (`ldtk_entity_contract.json`), not a Rust definition; the Rust name is `PortalGunSpawnSpec` -->
`axis_x: 1.0` with an attack edge every ten steps through the SHIPPED input road
(`drive_control_frame`). The gun is in hand on step 20 and shots are live
thereafter. Both hosts get the identical script, a pure function of the step
index, so the two still differ in exactly one thing and a disagreement would
still be about that. ⚠ And the first attempt named the
wrong room: `basement_npcs`'s `HazardBlock` is a SURFACE
(`SurfaceContact::ResetToSpawn`, `ldtk/src/surfaces.rs`), not a
`PlacementSchema::Hazard`; the authored identifier that becomes `feature.hazard`
is `DamageVolume`. A row's name is not a reading of what authors it, either.

⛔⛤ **OF THE THREE STILL SILENT, ONE COULD NOT BE PLACED BY ANY ROUTE — AND IT
IS NOW DELETED RATHER THAN RANKED.** `gravity.flip_switch`'s only mutable
writer, `gravity_flip_switch_system`, was registered in exactly one place in the
workspace and that place was inside a `#[cfg(test)]` module
(`gravity/lifecycle.rs`); the gravity plugin said so in its own words, and
nothing in the LDtk contract converted to the component. So the row was
rollback-registered twice (`component-clone` plus `require_rollback`) for a
component no shipped composition built, and **this ranking counted a writer no
production composition installs** — the same defect as crediting a road nobody
takes.

✅ **`Q137` RULED IT 2026-09-19 AND THE VERTICAL IS GONE.** Gravity SWITCHING
stays — the LDtk-authored `FlipGravity` / `SetGravity` switches and the
developer gravity controls are the product — and what went is the second,
unreachable ROAD to the same fact: the component, the system, both rollback
registrations, the per-tick view rebuild that could only produce an empty
vector, the render sync that despawned and rebuilt from it every frame, and the
exit-oracle row that named it by string. ⇒ The ranking is ELEVEN rows a route
could reach, and there is no twelfth. A later pressure plate is an INPUT into
`BaseGravity`, which every legitimate road already writes.

⛔⛤ **`portal.emission` WAS READ AS THE SECOND UNREACHABLE ROW, AND THAT
READING WAS WRONG IN BOTH HALVES — CORRECTED 2026-09-17, THE SAME DAY IT WAS
WRITTEN.** This section said the row wanted *"an AIMED script — fire at a
reachable wall, then walk into the aperture — which is a bigger instrument than a
room or a held direction"*. It wanted neither a gun nor an aim. What was missing
was the LOOK, not the road, and the road was a room already in the walk:

- **The aperture is AUTHORED.** `portal_lab` holds `a_purple`, a ground-ground
  pair at x 254..346 with normal `up`, and the player starts at x=80 — 174px to
  its left, on the same floor. Holding `axis_x: 1.0` walks the body into it and
  the transit emits at tick 50. The room was already the second walk; it was
  simply not driven, because it had been added to carry the AUTHORED
  `portal.placed` and nothing asked it to press anything.
- **The observation ladder could not see it.** `PortalEmission` lives
  `PortalTuning::emission_time_s` = **0.18 s, about 11 ticks**. The arm sampled
  at steps 0, 1, 30 and 120; the driven `portal_lab` walk carries the row at
  ticks 50-60, 68-78 and 140-150. Every window falls in a gap, and the widest gap
  was 90 ticks. ⇒ The arm now samples EVERY tick — a sample point chosen to land
  inside an 11-tick window would be a magic number that any movement-tuning
  change silently retires, and 121 censuses per host per room cost the arm about
  eleven seconds.

⛔ **BOTH HALVES ARE LOAD-BEARING, POISON-VERIFIED SEPARATELY.** Restoring the
four-rung ladder with the driven room in place loses exactly `portal.emission`
from the compared set; leaving the per-tick sampling and setting `portal_lab`
back to undriven loses exactly `portal.emission`. Neither change reddens
anything else.

⚠ **AND THE ORIGINAL OBSERVATION ABOUT THE GUN WALK WAS ACCURATE** — it was the
conclusion drawn from it that was not. `portal_bridge`, driven, with the gun in
hand, sampled at all 121 points, still carries no `portal.emission`: firing right
and holding right really does put both apertures on the same wall. The error was
reading "this walk does not reach it" as "no walk of this kind can".

⛔⛤ **AND `boss.death_animation` WAS THE LAST ONE, ON THE SAME MISTAKE, FOUND THE
SAME DAY.** This table used to hold one more row: *"a boss death, which is the
most expensive fixture of the set"*. **A boss does not have to die.**
`BossDeathAnimation::default()` is inserted AT SPAWN
(`crates/ambition_platformer2d_actor_spawn/src/actor_spawn/mod.rs:1141`), so any
room placing a `BossSpawn` carries the row — there are ELEVEN authored
`BossSpawn` placements across nine rooms, and `basement_boss` is now the fifth
walk. Removing that room loses exactly this row and nothing else, poison-verified.

⚠ **AND THE CARRIER IS CONSTANT, WHICH THIS PAGE WILL NOT OVERSTATE.** Measured
`(1, 0)` at every one of the 121 observation points, driven and undriven:
`remaining_s` stays `0.0` until something kills the boss, and 120 driven steps of
holding right do not. ⇒ What the arm compares for this row is the carrier's
PRESENCE and identity across two histories, not a varying float. **A real boss
death is still the stronger observation and still the expensive fixture.** What
was wrong was calling the row unreachable.

⇒ **TWICE IN ONE DAY A ROW READ AS UNREACHABLE BECAUSE THE SENTENCE DESCRIBED THE
EVENT THE FIELD IS NAMED FOR RATHER THAN THE CODE THAT INSERTS IT.**
`portal.emission` was said to want an aimed script and wanted a room already
walked plus a denser sample; `boss.death_animation` was said to want a boss death
and wanted a `BossSpawn`. Read the insert site, not the field name. ⇒ ONE row of
the twelve is left, and it is unreachable by any route (`Q137`); the authored
half of the list is spent.
⛔ Two Apps with different local histories are still not two peers — no
transport, no input exchange, no interleaving, no rebase — so this decides
whether a row's value depends on where the host has been, and nothing more. That
is the half `N2` cannot reach and a type census cannot see.

⚠ **AND THE CENSUS'S OWN LIMIT, STATED BECAUSE THE NUMBER LOOKS DECISIVE.** A
`&mut` census cannot see a component REPLACED by re-insertion, which is also a
write. I probed for that and my `insert(T…)` pattern returned 0 or 1 for every
one of the 13 — including components that must be inserted somewhere to exist at
all, since bevy inserts them inside tuples. So that probe measured its own
regex, not the code, and the 13 are "no mutable borrow", not "never written".
Closing that gap wants the same runtime instrument as S8: **is there a frame at
which this value differs between two saves of it?** `RollbackRestoreAudit`
answers that and no static scan can.

⛔ **THE FLOOR IS ON THE INTERSECTION, NOT ON ITS OPERANDS.** `sharp_rows` in
`scripts/measure_unchecksummed_rollback_rows.py` RAISES rather than returning an
empty list, because a join written against the wrong key — row names on one side,
type names on the other — returns zero rows, and a list of zero risks reads as
good news. An unresolved type (`?`) is also NOT promoted into the list, which
would inflate it with the instrument's own blind spots. Both are pinned by
`test_the_sharpest_list_is_an_intersection_and_excludes_each_operand_alone` and
`test_an_unresolved_type_is_not_promoted_into_the_sharpest_list`, each with its
negative case, and both were poisoned: each poison fires on its own arm only.

⭐⭐ **AND THE FIRST OF THE 25 IS NOW MEASURED, WHICH REQUIRED ADDING A ROAD
BECAUSE THE OBVIOUS INSTRUMENT IS BLIND TO ALL OF THEM.** The question the
ranking leaves is whether a row that is outside the peer checksum, read every
tick and float-bearing actually DIFFERS at a frame compared twice. The plan was
to point `RollbackRestoreAudit` at it, the way S8 does. That plan was empty:

> the audit compares `probes.census_all(world)`; a row registered through
> `rollback_component_clone` gets `ChecksumProbe::presence_for::<T>`, whose census
> is `census_presence`, which **hard-codes `xor: 0`**. It counts carriers. Every
> float in every carrier can drift by any amount and the census is byte-identical.

⇒ So "point the existing audit at the 25" was a plan to re-measure the carrier
count. ⛔ And there was no road to strengthen a probe either: `record_probe` is
private to registration, `RollbackChecksumProbes::probes` is a private field, and
editing the registration site changes that row's `detail`, which changes
`schema_dump()`, which changes `compute_schema_fingerprint`. **A purely local
DIAGNOSTIC property was welded to the identity two peers compare** — which is
`Q122` arrived at from the opposite direction, and a second, independent argument
for the same split.

`RollbackChecksumProbes::strengthen_with::<T>(projection)` is the road now.
Strength is owned by the probe collection at runtime and registration is how it
is INITIALIZED, not where it is decided — one owner, not two, and after the call
`strength_tally()` and `presence_only_type_names()` both report `T` as a value
probe because it now is one. Nothing a probe does reaches the GGRS aggregate,
which is what makes strengthening one free of peer consequences. It returns
`false` for an unregistered type rather than censusing nothing.

✔ **`item.ground_item` REPRODUCES.** The largest writer of the 12 — seven `&mut
GroundItem` sites, carrying `pos`, `vel` and `half_extent` as `Vec2`, a moving
physical object whose position decides whether a body can pick it up. Held by
`a_falling_ground_item_reproduces_its_value_across_every_resimulation`
(`game/ambition_app/tests/does_a_presence_probed_row_move_when_its_value_does.rs`):
the authored object in `blink_run` is picked up and Z-dropped on the production
road, and across the fall the strengthened probe took **3 distinct censuses at the
3 frames the audit compared, with 0 divergences.** Its control — the same room,
the same release, a projection that reads the carrier and returns a constant —
takes exactly 1 census and also reports 0, which is what separates "the value
reproduced" from "the probe never saw the value".

⛔⛤ **AND THE FLOOR THAT ARM NEEDED IS A NEW LESSON, NOT A REUSED ONE:
`resimulations > 0` AND "THE VALUE WAS MOVING" ARE DIFFERENT CLAIMS.** The window
is 24 steps and the audit compared THREE frames. A ground item falls for a couple
of frames and settles, so "the value moved during the window" is satisfied by a
run in which every compared frame holds the object at rest — and the arm would
then have reported *"this value reproduces across a rewind"* about a stationary
object, with a floor in place and a control passing. The number that closes it is
how many different censuses the probe took **at the frames the audit compared**,
which `RollbackRestoreAudit::distinct_censuses_across_compared_frames_of::<T>()`
now answers; the audit records which frames it resaved for exactly this.

⚠ **THE FIRST RUN OF THAT FILE MEASURED A POPULATION OF ZERO AND REPORTED IT
CLEAN.** Pointed at `combat_calibration_lab` — the room every other rollback arm
uses — it printed `carriers=0` at all 40 steps, 148 saves, 108 replay-comparable,
and *"no component changed across a save/load of the same frame"*. That room
authors no ground item. The only reason it was not read as an answer is that the
probe printed the carrier count beside the verdict. `blink_run` authors exactly
one, which is the room the arm uses.

⇒ Both arms were poisoned, each firing on its own assertion and nothing else: the
good arm run with the constant projection fails the compared-frames floor, and run
with a projection that cannot reproduce fails the `diverging == 0` assertion. Two
poisons, two different assertions, zero compile errors in either run, and the file
restored byte-identical after each.

✔ **AND `actor.animation_facts` REPRODUCES TOO — THE SHARPEST MEMBER OF THE 25,
BY ITS OWN WRITER'S DESCRIPTION.** `advance_body_anim_overlays`' doc says so
without being asked: *"Measured before the move: `BodyAnimFacts` is
rollback-registered as `actor.animation_facts`, so everything here writes
**canonical simulation state restored on every rewind** — this must run in the
deterministic sim, and must not be reclassified as presentation on the strength
of the field names."* So: `f32` timers, decayed by `frame_dt` every tick by a sim
system, restored on every rewind, declared canonical beside the code that writes
them — and outside the checksum two peers compare. Float accumulation is the
exact failure mode a per-tick decay has and a carrier count is blind to all of
it. Measured: **2 distinct censuses at the 116 frames the audit compared, 0
divergences**, against a constant-projection control at 1 census. Its own doc
also bounds the consequence — *"what it does NOT do is affect simulation
GEOMETRY: authored attack volumes resolve against an animation row chosen by
`attack_intent_animation(intent)`, a match on the attack INTENT, which never
consults these timers"* — which is a real limit on how bad a drift would be, and
not a reason the value reproduces.

⛔⛤ **AND THE FIRST WINDOW FOR THAT ROW HAD THE SUBJECT FROZEN AT ZERO, WITH
EVERY OTHER NUMBER LOOKING EXCELLENT.** Holding `attack` for 40 steps gave 148
saves, 108 replay-comparable, **36 compared**, four carriers throughout — and ONE
distinct census. Read without the floor that is *"`BodyAnimFacts` reproduces
across 36 comparisons"*. The probe then measured four inputs across 60 steps
each: `attack` held, `attack` pressed on the 1-in-12 edge that
`game/ambition_app/tests/a_move_keeps_its_occurrence_across_a_rewind.rs` uses
to start several moves,
jump-and-land, and run-and-jump. **Under both attack inputs every field of every
carrier read exactly `0.000` at every step.** Only a landing moved anything, and
`land_anim_timer` is non-zero for about two frames per touchdown — so the window
had to be 120 steps with a jump every eight.

⇒ **THE GENERAL FORM, AND IT IS THE FLOOR LESSON ONE LEVEL SHARPER: THE VERB THAT
MOVES A VALUE IS A MEASUREMENT, NOT A GUESS FROM THE FIELD NAMES.**
`slash_anim_timer` was the obvious target for an attack press and it never left
zero in this composition. A window chosen from a field name, with 36 clean
comparisons and four carriers to back it, produces a verdict about rest that
reads exactly like a verdict about motion.

⛔⛔ **AND THE LIMIT ON BOTH RESULTS, WHICH IS LARGE ENOUGH THAT IT CHANGES WHAT
THE REMAINING TEN ARE WORTH.** What that instrument measures is whether a value
survives **one machine rewinding itself**. That is not the question S7's rows are
dangerous for.

`Session::SyncTest` is constructed in exactly ONE place in this workspace
(`crates/ambition_platformer2d_rollback_ggrs/src/session.rs`), and `Session::P2P`
appears exactly once, in a match arm reading `confirmed_frame()` — **no P2P
session is ever built.** So every rollback measurement in this repository,
including these two, is a local resimulation comparison.

⇒ A value OUTSIDE the peer checksum can be perfectly reproducible under local
resimulation and still differ between two peers, because **nothing compares it
between peers at all.** The two questions are:

| question | what answers it | the two rows' verdict |
|---|---|---|
| does a rewind restore this value correctly? | `RollbackRestoreAudit` + a value probe | ✔ yes, both of them |
| do two peers agree about this value? | no SESSION compares it; a two-HOST arm can | ⚠ see below — measured for `actor.animation_facts`, still unmeasured for the rest |

So `item.ground_item` and `actor.animation_facts` are cleared of a LOCAL RESTORE
defect, which is a real class and was worth ruling out — the audit's five other
users exist because that class has bitten. They are **not** cleared of the thing
S7 is about. A per-tick float decay that two Apps compute differently, for any
reason, diverges silently forever, and a green SyncTest is exactly what that
looks like from inside one App.

⇒ **THAT REDIRECTS THE REMAINING TEN FROM A GRIND TO A DECISION.** Measuring each
of them the same way would produce ten more "clean under local resimulation"
verdicts that do not answer the question, and would read in this page as ten rows
cleared. The question the 25 actually pose is ID-PEER's acceptance test — two Apps
with different prior local histories entering the same route and agreeing on
mechanical state. **`Q128` and the absent P2P session are the same blocker wearing
two names** for the TIMELINE half, and the 25 are a ranked list of what a real
session would need to compare.

⭐⭐ **BUT THE STATE HALF OF THAT ACCEPTANCE TEST CAN BE ASKED TODAY, AND THIS
PAGE SAID IT COULD NOT — CORRECTED 2026-09-16.** The table above read
*"unmeasured, and unmeasurABLE here"*, which conflated two different absences: no
P2P SESSION can be built (`SyncTestSession` is the only one this workspace
constructs), but two APPS WITH DIFFERENT LOCAL HISTORIES can, and comparing what
they compute is a different KIND of evidence from replaying one App against its
own past. `two_local_histories_compute_the_same_mechanical_values`
(`game/ambition_app/tests/shell_host_lifecycle.rs`) launches the shipped Ambition
route first in one host and third in another — scope `0` / epoch `1` against scope
`2` / epoch `3`, asserted first so the comparison is controlled — and compares
`BodyAnimFacts` BITWISE, keyed by canonical `SimId`, over 120 steps. It agrees.

⚠ **WHAT IT IS NOT.** No transport, no input exchange, no interleaving, no
timeline rebase — so it does not answer `Q128` and does not retire N2. And its
strength is entirely in its floors: the census must take more than ONE value
across the window, or it is a verdict about rest wearing a verdict about motion,
which is the trap this page recorded one section up. The ground items in that
route ARE at rest — measured — so their half of the arm is labelled as witnessing
CONSTRUCTION agreement rather than per-tick agreement.

⛔⛤ **AND THE FIRST ATTEMPT AT IT WAS VACUOUS.** Two `Platformer2dSimHarness`
instances built in one process were compared and agreed — then their tokens were
printed and both read `SessionScopeId(0)` at tick 1. A fresh App is a fresh
counter, so a bare sim harness cannot CARRY a differing history; that comparison
was one input against itself. ⇒ **The differing history has to live inside ONE App
that has been somewhere first**, which is why this arm is in the shell-host file
and not beside the probe that measured the same rows locally.

⇒ So the remaining ten are still not ten more LOCAL resimulation measurements.
They are ten more rows that the two-host arm above could be extended to, one at a
time, each costing the same thing the first one did: a projection, and a verb
that moves it.

ⓘ The instrument stays, because it is cheap and its class is real: one generic
function (`measure::<T>`) plus four floors
(`the_reading_is_about_the_subject`), so a row costs a projection, a room, and an
input measured to move it. Spend it when a row is SUSPECTED, not to walk the list.

### S8 — the hashed entries written from a schedule that never rewinds

✔✔ **CLOSED 2026-09-16, AND RE-VERIFIED 2026-09-17: NO REGISTERED TYPE IS
WRITTEN OUTSIDE THE REWINDING SCHEDULE, AND THE SAVE IS NO LONGER PINNED.** The
repair was the three live→save mirrors — `persist_inventory_to_save`,
`persist_occurrence_horizon_to_save`, `persist_minted_item_horizon_to_save` —
registering through `app.sim_schedule()` instead of top-level `Update`, so a
rewind replays them and one historical frame stops seeing two different saves
(`f95d49ce6`). ⛔ **NOT by unhashing `AmbitionGameSave`**, which is what this
section recommended on both sides for days; the writer census below is why.

Three arms hold it, and **all three were INVERTED rather than deleted** even
though both this file and the arms' own docs said to delete them on this
outcome — deleting them would have retired the only instruments that can notice
the mirrors drifting back out of the rewind window:

| arm | asserts today | was |
|---|---|---|
| `no_registered_type_is_written_outside_the_rewinding_schedule` | the outside set is EMPTY | `exactly_one_…`, and it was the save |
| `no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves` | the diverging set is EMPTY, floored on the projection varying so an empty set cannot pass vacuously | `exactly_one_hashed_entry_diverges_…` |
| `the_saves_hashed_snapshot_tracks_the_frames_it_is_compared_at` | the save's projection takes **≥ 50** distinct censuses across the compared frames | `…_holds_one_value_across_every_compared_frame` |

⇒ **The floor is 50 and not `> 1` on purpose.** The pinned regime read 1 idle and
2 with an acting agent, so `> 1` would accept the effectively-frozen state the
arm exists to refuse; the repaired regime read 236 against neighbours' 238. ⭐ An
empty divergence set from a PINNED projection is `f = const`: every equality arm
goes green the moment the projection stops responding to the state it covers, in
the direction that looks like success. That is why the un-pinning is asserted
beside the emptiness rather than inferred from it.

✔ Verified 2026-09-17 at `b8ad12f2a`: all three pass
(`cargo test -p ambition_app --test app_it -- <name>`), and the un-pinning
re-derived the same day by
`probe_whether_the_saves_snapshot_tracks_its_frame_after_the_window`:

```
coverage: 948 save(s) censused (708 repeats, so replay-comparable), 236 compared
distinct save censuses across COMPARED frames:  236
save census at the FIRST compared frame   (2):  0x8f605a278dac557d
save census at the LAST  compared frame (239):  0xd93f11239a89c3b1
CONTROL — types whose census moved: 14, nine of them at 238
```

⛔ `0x8f605a278dac557d` is the constant the pinned regime held at EVERY compared
frame; it is now only the first one. That is the tell reversing, read off the
same instrument that found it.

**What follows is the baseline record: the measurement chain that found it.** It
is kept because the eliminations in it are reusable and several of them were
wrong in instructive ways; every claim in it is about the world BEFORE
`f95d49ce6`.

⛔⛤ **FOUR HASHED ENTRIES WERE WRITTEN FROM `Update`, AND ONE OF THEM WAS A
PROVEN SYNC-TEST DESYNC.** Measured 2026-09-16 by crossing
`check_rollback_mutators_run_in_sim.py`'s offender list against the registration
KIND of each type it names. The guard reports systems; it does not ask whether the
value they touch is compared between peers, and that is the question that turns a
placement note into a defect.

| type the `Update` system writes | registration kind | hashed? |
|---|---|---|
| `AmbitionGameSave` | `resource-clone-custom-checksum` | **YES** |
| `NewGameResetRequested` | `resource-canonical` | **YES** |
| `CustodyBaseline` | `resource-clone-custom-checksum` | **YES** |
| `OccurrenceBaseline` | `resource-clone-custom-checksum` | **YES** |
| `OwnedItems` | `resource-clone` | no |
| `SaveRestored` | `resource-clone` | no |

Eight systems, six types, four hashed. The writers AS MEASURED THAT DAY:
`persist_inventory_to_save`, `persist_minted_item_horizon_to_save`,
`persist_occurrence_horizon_to_save` and `dispatch_pending_dialog_requests`
(`AmbitionGameSave`); `adopt_occurrence_checkpoint_from_save` (`CustodyBaseline`,
`OccurrenceBaseline`); `grid_menu_action_activated` and
`kaleidoscope_menu_action_activated` (`NewGameResetRequested`, `OwnedItems`);
`complete_durable_restore` (`SaveRestored`).

✅⛤ **AND `AmbitionGameSave` IS OUT OF THAT LIST — re-run 2026-09-18, the table
above is history and is dated for that reason.** All four of its `Update` writers
are gone from `check_rollback_mutators_run_in_sim.py`'s offender list: the three
mirrors moved into the sim schedule and `dispatch_pending_dialog_requests` stopped
taking the resource (the count is now
`count_the_dialogue_visit_when_a_conversation_opens`, in the schedule, keyed on
`ActiveConversation`'s opening tick). What the guard reports today is **8
acknowledged offenders over 523 mutating systems** — 518 when this paragraph was
written earlier the same day; the population grew as the guard learned two more
ways a write can be spelled (an exclusive-world body write, then a QUALIFIED
schedule label, which alone had been hiding 39 registrations) — none of them
writing the save:
`adopt_occurrence_checkpoint_from_save`, `complete_durable_restore`,
`compute_music_intent`, `grid_menu_action_activated`,
`kaleidoscope_menu_action_activated`, `portal_dev_toggle_system`,
`reconcile_roster_with_frozen_topology`, `sync_ldtk_level_set`.
⇒ Of the four hashed entries, `AmbitionGameSave` is repaired and the two
baselines are the open half — a LIFECYCLE question rather than a placement one,
owned by
[DURABLE-HORIZON-CHECKSUM](../queue.md#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update)
and ruled on by
[Q135](../awaiting-maintainer-decision.md#q135--should-ggrs-start-before-the-durable-restore-has-finished).
⚠ The three-part predicate below is the durable part of this section and is
unchanged by any of that.

⚠ **BEING HASHED AND WRITTEN FROM `Update` IS NOT SUFFICIENT, AND THAT IS
MEASURED, NOT ARGUED.** The predicate has three parts:

1. the entry is hashed (`feeds_peer_checksum()`),
2. its writer is outside the rewinding schedule,
3. **its value actually DIFFERS at a frame that gets compared twice.**

`NewGameResetRequested` satisfies 1 and 2 and does NOT desync: set from `Update`
mid-window, the room is not rebuilt, 7 of 7 roster entities survive and
`session_health` is clean for 180 frames — because the flag is put back before it
is taken. ⇒ **A checksum cannot disagree about a value that was put back before it
was taken.** `AmbitionGameSave` satisfies all three and desyncs within six ticks.

⇒ So the four are a FLOOR OF CANDIDATES, not a count of defects. The count as of
2026-09-16 is **three measured defects and one clean with a stated mechanism** —
and it got there in three readings, each of which raised it, because the first two
counted the fixture and not the code. The surviving clean entry is
`NewGameResetRequested`, and what makes it different is that its clean verdict
rests on a MECHANISM (*put back before it is taken*) rather than on an absence of
findings.

⛔⛤ **UPDATED 2026-09-16: BOTH BASELINES ARE MEASURED DEFECTS, AND "MEASURED
CLEAN" WAS "MEASURED EMPTY" EACH TIME.** The runs that cleared them ran against a
harness with no save file, so `adopt_the_ledger` wrote back the same empty value
it read and condition 3 — *its value actually DIFFERS at a frame compared twice* —
was never met by the fixture rather than never met by the code. Given a save that
says something in BOTH halves of the durable horizon, staged from inside the
rewinding schedule at tick 40:

    baseline_rows / custody_rows              1 / 1
    written_outside_the_rewinding_schedule()  ["...continuity::OccurrenceBaseline",
                                               "...custody_horizon::CustodyBaseline"]
    session_health()                          Err("checksum mismatch at frames [38, 39, 40]")

⛔ **AND `CustodyBaseline` TOOK ONE MORE READING THAN `OccurrenceBaseline`, FOR A
REASON WORTH KEEPING.** The staging system seeded occurrences and passed
`Vec::new()` for custody, so `adopt_occurrence_checkpoint_from_save` — which hands
BOTH baselines to `adopt_the_ledger` — wrote the custody half back unchanged. The
save file existed, the load ran, the writer executed, and the resource still did
not move. ⇒ **A fixture can put pressure on one field of a pair and none on the
other, and the report does not say which.** The old one-member reading is
reproducible on demand: emptying the custody seed again prints
`outside=["...OccurrenceBaseline"]` with the second member gone.

⇒ Held by `probe_what_a_mid_session_load_writes_outside_the_rewinding_schedule` in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`, `#[ignore]`d
because it demonstrates an unfixed defect. It now asserts, rather than prints:
its PREMISE (a row landed in both baselines, so a revert to `Vec::new()` fails
loudly instead of quietly narrowing the subject) and the DEFECT (both names are in
the outside set, so a fix reds the arm and forces the inversion). Both were
poison-verified — the premise fires with `(occurrence=1, custody=0)` and the defect
loop names the missing member. The ruling it wants is
[Q135](../awaiting-maintainer-decision.md#q135--should-ggrs-start-before-the-durable-restore-has-finished).

⭐⭐ **AND THE OTHER TWO WERE MEASURED WITHOUT BEING AIMED AT, WHICH IS WHAT A
PER-ENTRY CENSUS BUYS.** The arm now called
`no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves` then asserted the
diverging set was EXACTLY `{AmbitionGameSave}`, over 364 probed
entries. `CustodyBaseline` and `OccurrenceBaseline` are in that population and did
not diverge — ⛔ **and 2026-09-16 showed why, which is not the reason this
paragraph gives: both were EMPTY, and a load that seeds BOTH halves makes both
diverge and desync.** What follows is the original reasoning, kept because the
window it describes is real and the conclusion drawn from it was not: in a window
where `AmbitionGameSave` itself was being rewritten from
`Update` on every frame, which is the sharpest pressure on them available, because
`adopt_occurrence_checkpoint_from_save` READS `AmbitionGameSave` and writes both
baselines from it. A chain of bag → save → baseline was live and only the first
link moved.

⚠ **THE LIMIT OF THAT NEGATIVE, STATED SO IT IS NOT CITED FURTHER.** The audit
reports which entries DIVERGED; it does not report whether
`adopt_occurrence_checkpoint_from_save` ran at all in that world — it early-returns
on `restored.0` or on an empty primary-body query, and "clean" and "never
executed" are the same reading here. ⇒ What is unmeasured is a window in which the
occurrence ledger or custody themselves move. The instrument is the same one:
`RollbackRestoreAudit::enabled()` read inside the live frames. See
[ROLLBACK-BAG-DESYNC](../queue.md) and `Q129`.

⛔⛤ **AND THE OBVIOUS REPAIR IS THE LARGEST ONE, MEASURED BY WHAT IT STOPS
CHECKING.** "Take `AmbitionGameSave` out of the peer checksum — a save file is not
simulation authority" was the recommendation on both sides of this until the
writer census landed: of **19 systems taking `ResMut<AmbitionGameSave>`, 13 are in
the SIM schedule** — quest advances, boss encounter progress, switch activations,
shrine heals, cutscene ticks, wave encounters, flag effects. Removing the entry
from the checksum would stop comparing all thirteen, in the schedule where the
comparison is doing real work. ⇒ Deriving the save inside the sim schedule is the
honest repair, and the three `persist_*` mirrors are the only reason it is not
there already. Measured by CalculexAmbition and filed with the table in `Q129`.

⚠ **TWO INSTRUMENT NOTES FROM THAT CENSUS, BOTH OF WHICH WOULD HAVE MOVED THE
COUNT:** a name inside `.after(...)` is an ordering EDGE, not a registration, so a
classifier that counts it attributes a system to whatever schedule its neighbour
is in (`heal_save_shrine_system` has an `.after()` mention and a real
`add_systems` elsewhere). And the three `persist_*` mirrors landing on the
`Update` side is the POSITIVE CONTROL: a classifier that put them anywhere else
would be wrong about the very systems the question is named for.

⚠ **AND THE REPRODUCTION NEEDS A SUSTAINED CHANGE, NOT A CHANGE.** A `SimTick`-gated
grant firing ONCE at tick 20 runs 240 steps clean with health `Ok` while the bag
moves 3 → 4. Only the per-tick change reproduces — so an arm written around a
single grant reports no divergence, which reads exactly like a repaired world. The
pinned arm floors both audits on `resimulations > 0` for that reason.

⛔⛔ **AND THE MEASUREMENT THAT CHANGES WHAT S8 IS ABOUT: `AmbitionGameSave`'S
HASHED SNAPSHOT IS PINNED, NOT MERELY LATE.** Measured 2026-09-16 by
`probe_whether_the_saves_snapshot_tracks_its_frame_after_the_window`
(`game/ambition_app/tests/which_hashed_entry_moves_when_the_bag_does.rs`), with a
per-tick grant gated to start at tick 4 so the run stays healthy for 240 steps:

```
end tick 241, health Ok, 948 saves censused (708 replay-comparable), 236 compared
save census at the FIRST compared frame (2):   count=1 xor=0x8f605a278dac557d
save census at the LAST  compared frame (239): count=1 xor=0x8f605a278dac557d
save checksum LIVE at the end:                          0x722bb3a9f4b6d72c
items mirrored into the save by then:                   247
```

⇒ **At every one of 236 frames GGRS saved twice, the save resource held the SAME
value — the early-game one — while the live save had moved to a completely
different checksum with 247 items in it.** The registered projection is
`AmbitionGameSave::checksum` itself (`rollback_resource_clone_checksum::<…>(…,
AmbitionGameSave::checksum)`), which serialises the whole save to RON, so this is
not a narrow projection missing the items.

⛔⛤ **THE CONTROL IS THE ABSENCE OF THE SUBJECT AND IT PASSES DECISIVELY.** In the
same run, `types_whose_census_moved_across_compared_frames()` reports **13 types
whose census moved, nine of them taking 238 distinct values across 236 compared
frames** — a new value essentially every frame: `SimTick`, `BodyKinematics`,
`BodyLifetime`, `SweepSample`, `MotionModel`, `CenteredAabb`, `ActorPose`,
`GameplayElapsed`, `PlayerProjectileState`. So "the save's census never moved" is
a fact about the save and not about the audit, which was recording a new value
per frame for nine neighbours at the same instants.

⭐⭐ **AND THE PINNED VALUE IS A NUMBER THREE INSTRUMENTS NOW AGREE ON.**
`0x8f605a278dac557d` is exactly what CalculexAmbition's independent per-tick
sampler read as the LIVE save checksum at ticks 15–20 in a separate session, and
`0xce4e4758…` — the constant replay value this file's own audit reported for
frames 2–4 in the desyncing variant — was their tick-1 reading by the same route.
Two sessions, three instruments, the same constants.

⇒ **SO A "CLEAN" RUN HERE IS NOT EVIDENCE THE SAVE IS BEING COMPARED CORRECTLY; IT
IS EVIDENCE THAT WHAT IS COMPARED IS FROZEN.** Two saves of one frame cannot
disagree about a value that is the same at every frame. That reverses the reading
of every clean result in this neighbourhood, including the ones above: the entry
the divergence arm pinned was the entry that diverges in the ONE window — the
first three ticks — where the snapshot was not yet pinned. ⇒ That is the reading
that made the un-pinning floor in
`the_saves_hashed_snapshot_tracks_the_frames_it_is_compared_at` a separate
assertion instead of a corollary of the empty set.

⭐⭐ **AND S8'S POPULATION IS NOW MEASURED RATHER THAN READ OFF EIGHT SYSTEMS'
SCHEDULES — THE ANSWER IS ONE, AND IT IS THE SAVE.** This section found its four by
reading `add_systems` calls and registrations, which is careful work that a
forwarder, a set or a `cfg` can hide from. There is a direct measurement: the GGRS
advance runs in `PreUpdate` (`run_ggrs_schedules`), so censusing the world at the
END of the advance and again at the end of the frame makes the difference exactly
what `Update`, `PostUpdate` and `Last` wrote. Over 240 frames with an acting
agent:

> **1 registered type is written outside the rewinding schedule, and it is
> `ambition_persistence::save::AmbitionGameSave`.** 240 live comparisons; nothing
> else, hashed or not, differs between the two censuses.

⇒ The other three this section names are explained by the instrument's two STATED
blind spots rather than by disagreement: `NewGameResetRequested` is put back
within the frame (which is also why it satisfies the first two conditions and does
not desync), and `CustodyBaseline` / `OccurrenceBaseline` are measured EMPTY for
the whole run, below — ⛔ which is a statement about the FIXTURE, not about the
code: seed BOTH halves of the durable horizon and BOTH baselines appear in this
very set. Held by the arm now called
`no_registered_type_is_written_outside_the_rewinding_schedule`, which then
asserted the set was exactly `{AmbitionGameSave}` and whose positive control was
that the save MUST appear — a known answer established by a different route,
because an empty set reads exactly like a clean world. ⚠ The repair emptied the
set, so the arm asserts the emptiness now; the control is what keeps that empty
answer meaningful.

⚠ **THE POPULATION IS "TYPES WHOSE PROBE CAN SEE A VALUE CHANGE", NOT "ALL
STATE".** A presence probe counts carriers and is blind to a value, and a type that
is not rollback-registered cannot appear however it is written —
`SeatControlFrameModes` and `PlayerDamagePolicy` are both written from `Update`,
read by sim systems, and invisible here because neither is registered. That is
`SETTINGS-ROLLBACK`'s row, not a hole in this one.

⛔⛤ **THE FIRST VERSION OF THAT INSTRUMENT SAID 29 OF 144 AND WAS WRONG IN THE
ALARMING DIRECTION.** It compared the live world against the last SAVED census —
but GGRS saves a frame BEFORE advancing it, so the saved census is the START of
the last advanced frame while the live world is its END, and every
per-tick-changing type differed by construction. The list included `SimTick`,
`BodyKinematics`, `MotionModel` and `SweepSample`. ⇒ **The number was seven times
this section's hand-read four, which is how a broken instrument announces itself as
a discovery.** What caught it was reading the LIST and seeing types whose answer
was already known — not reading the number, which was the most interesting number
of the day.

✔ **AND S8'S OWN STATED LIMIT IS NOW CLOSED, WITH THE ANSWER BEING "NO SUBJECT"
RATHER THAN "CLEAN".** This section recorded that `CustodyBaseline` and
`OccurrenceBaseline` were measured clean *"without being aimed at"*, and named the
limit: the audit reports which entries DIVERGED, never whether the capture ran at
all. Measured 2026-09-16 by `probe_whether_s8s_baselines_are_quiet_or_frozen`
(`game/ambition_app/tests/how_much_of_the_peer_checksum_actually_varies.rs`), over
240 steps with an acting agent:

```
CustodyBaseline rows 0 -> 0 ; OccurrenceBaseline rows 0 -> 0
CustodyBaseline      live 0xa8c7f832281a39c5 -> 0xa8c7f832281a39c5   censuses: 1
OccurrenceBaseline   live 0xa8c7f832281a39c5 -> 0xa8c7f832281a39c5   censuses: 1
```

⇒ **Both baselines are EMPTY for the whole run and their live checksums never
move**, so their clean verdict is about a subject that was never captured. There
is no defect VISIBLE TO THIS INSTRUMENT — and no evidence either, which is the
distinction the limit was pointing at. ⛔ **And the later seeded probe settled
which of the two it was: there IS a defect, in both.** So "no subject" was the
right verdict about this run and the wrong thing to carry forward as a property
of the resources. ⛔ The tell that made it findable is worth keeping: **two
structurally different types produced the SAME digest**, which is what an
empty-collection projection does, and a digest read without its population would
have looked like two independent confirmations.

⇒ So of S8's four `Update`-written hashed entries, the paired census now classifies
all four: `AmbitionGameSave` moves twice while its live value moves 247 times
(effectively frozen, below); `NewGameResetRequested`, `CustodyBaseline` and
`OccurrenceBaseline` are constant under both idle and play, and the two baselines
are additionally measured EMPTY, so "constant" is not evidence about them either
way. ⇒ **Two of those three were later shown to be defects under a seeded load;
only `NewGameResetRequested` survives, and on a mechanism rather than on a
constant.**

⛔⛤ **A CORRECTION TO THE SENTENCE ABOVE, MADE THE SAME DAY AND BY THE NEXT
MEASUREMENT: "EXACTLY ONE VALUE" IS AN IDLE-RUN FACT.** The run that produced it
steps with `AgentAction::default()` — no input at all. Re-run with an acting agent
(run, jump, attack on an edge) over the same 236 compared frames, the save's
census takes **2 distinct values, not 1**. So it is not literally frozen. It is
effectively frozen, and the honest comparison is the one against its neighbours in
the same run:

| entry | distinct censuses over 236 compared frames |
|---|---|
| `SimTick`, `BodyKinematics`, `MotionModel`, `ActorPose`, `CenteredAabb`, … (10 of them) | **238** |
| `ActorTarget` | 228 |
| `MovePlayback` | 214 |
| `BodyMelee` | 65 |
| **`AmbitionGameSave`** | **2**, while its live value reached 247 mirrored items |

⇒ The finding survived the correction and was better stated by it: a hashed entry
whose live value changes on every one of 240 ticks contributed **two** values to
what two peers would compare, in a run where ten of its neighbours contributed
238. The arm pinned the idle number because it was the reproducible one. ✔ It
reads 236 now and is floored at 50 — see the header of this section for why that
floor, and not `> 1`.

✔ **THE LINK TO THE PEER CHECKSUM IS CLOSED BY CONSTRUCTION, NOT BY A SECOND
MEASUREMENT — AND THAT IS THE STRONGER CLOSURE.** The worry was that *"the probe
census is constant"* and *"the peer checksum contribution is constant"* are two
sentences. They are not two samples. `install_resource_clone_checksum`
(`ambition_platformer2d_rollback_ggrs/src/registration.rs`) installs both from the
SAME `checksum` argument, in the same call:

```rust
RollbackApp::rollback_resource_with_clone::<T>(app);
RollbackApp::checksum_resource(app, checksum);          // the GGRS aggregate
record_probe(app, ChecksumProbe::new(type_name::<T>(), move |world| {
    census_resource_with::<T>(world, checksum)          // the probe
}));
```

⇒ So the probe's `xor` **is** the value the aggregate folds — one function,
`AmbitionGameSave::checksum`, applied to one world. `record_saved_census` is
registered `add_systems(SaveWorld, …)` and `checksum_resource` adds its system to
the same `SaveWorld` run, which builds the snapshot; nothing there mutates
gameplay state, it copies it. A probe that agreed with the aggregate would only
have re-measured the same call.

⚠ **THE ONE CONDITION A FUTURE READER SHOULD CHECK, stated rather than left
implicit:** the two systems are unordered within `SaveWorld`. That is immaterial
while nothing in `SaveWorld` writes the save, and it stops being immaterial the
moment something does.

⇒ This is `Q129`'s subject and CalculexAmbition owns that row. The shape it
changes: the question was *"must the save file be part of what two peers agree
on"*, and the measured answer today is that it **is in the contract by
registration and out of it in effect** — a hashed entry contributing a constant.
Neither "in" nor "out" describes that, and a ruling that says "keep it in" would
be ratifying something that is not happening.

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
