# The queue — live execution order

This file is the **current executable engineering queue**. It is not a work log,
review transcript or archive. Git history owns completed investigations. Durable
design and measurements belong in the linked owner document. Product decisions
belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

A row remains here only while an engineer can act on it. When it closes, keep a
short receipt only where another open row depends on that fact; otherwise remove
it.

## P0 — architecture and correctness

### A10 — candidate world / last-good-world publication

**Owner:** [construction and reconstitution](engine/construction-and-reconstitution.md).

**CURRENT INVARIANT.** A failed candidate world leaves the currently playable
world N intact, at BOTH scopes and UNCHANGED — not merely playable. A candidate
N+1 is prepared and verified off to the side; only a validated candidate becomes
authoritative; N is retired only after that publication succeeds.

**CURRENT HEAD BEHAVIOUR (2026-09-15).** **CLOSED.** Every road that can change
the authoritative world crosses its own publication's verdict — the room
transition, the reset, the death reconstruction, the shell handoff and the dev
LDtk reload, each with a refusal arm and an admission control in `app_it`. There
is ONE road into a live session and ONE publication authority per level, with no
mode flag: the candidate-bracket selector is deleted rather than frozen on. A
verified publication also FREEZES what it owes the world outside its own
population, so a room published inside a pending candidate session announces
nothing to the live one until that session is admitted.

The five ownership contracts named by the 2026-09-15 holistic audit:

| # | Contract | State |
| --- | --- | --- |
| 1 | Nested non-entity/effect ownership | CLOSED — `FrozenPublicationEffects`, consumed by `finalize_room_publication` |
| 2 | Exact per-publication custody ownership | CLOSED — `CustodyHandoffs` on the exact `RoomPublication` |
| 3 | Candidate-owned minted reconstruction input | CLOSED structurally and behaviourally |
| 4 | True pre-construction refusal | CLOSED — `construct_room_candidate`, witnessed by an insertion hook |
| 5 | One exact verification-and-application target | CLOSED — one entity through both ends, every sink preflighted |

**NEXT IMPLEMENTATION STEP.** None. Post-A10 demolition is the active lane:
delete the mechanisms A10's replacement made dead, and keep the consolidation
control plane matching source. ⛔ Not part of A10: peer-stable identity
(ID-PEER's row below), the defensive `DepartureAuthority::Custodian` fallback,
and the refused-door player signal — the last is presentation policy awaiting a
product ruling, not last-good-world correctness.

**ACCEPTANCE CRITERIA.** A production composition demonstrating (a) failed
candidate construction/publication leaves the last-good world playable AND
unchanged, and (b) successful replacement validates N+1 before retiring N. ⇒
**MET at both scopes**, in `game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs`
and `death_restores_the_checkpoint.rs`.

**ACTUAL BLOCKER.** None.

**RECEIPTS.** The measurements, the witnesses, the three instrument defects that
made two of them pass before they worked, and the investigation that closed the
session scope are in the owner document — see its *"2026-09-15 holistic audit"*,
*"decisive acceptance statement"* and *"How the session scope got closed"*
sections. ⛔ They are NOT repeated here: this row had grown to 694 of the queue's
1048 lines, which made the live executable queue two thirds one closed campaign's
diary.

### ID-PEER — remove host-local lineage from peer-stable mechanical identity

**Owner:** deterministic identity / rollback architecture; see the identity map in
[`consolidation/architecture-census.md`](consolidation/architecture-census.md).

**Current state (2026-09-16): NINE OF THE TEN NAMED ROADS ARE CLOSED, and the
tenth is the absolute `SimTick`, which is netcode and wants a maintainer decision
before anyone starts — asked as `Q128`.** A count rather than a completeness word, deliberately: a
tenth road found tomorrow makes this row "nine closed, a tenth found" instead of
making it false.

⛔ **THE FIRST ATTEMPT AT THREE OF THEM REPLACED ONE HOST-LOCAL TERM WITH
ANOTHER**, which two GPT architecture reviews (2026-09-15, 2026-09-16) found in
turn — the activation tick for the session id, then the session-relative ordinal
for the activation tick, each correct one layer up and wrong one layer down. The
table below is written to be re-checkable rather than reassuring, and every row
names the arm that holds it.

⭐ **THE TWO THAT CLOSED LAST CLOSED WITHOUT A NEW AUTHORITY, and that is the
pattern worth carrying into whatever is next.** The cross-session match identity
needed a stale stamp to be impossible, not a longer checksum — and the one owner
of "resources that must not survive a session" already existed with an exhaustive
destructure; four types were simply not in it. The session root's identity needed
no peer-stable session identity at all — the local count was disambiguating
nothing, because `shell_host_lifecycle` had been asserting `session_roots == 1`
in green for weeks. ⇒ Ask who ORDERS and who OWNS a thing before designing a type
to carry it.

| road | state |
|---|---|
| smash random-roster seed | **CLOSED** — `agreed_match_seed` hashes the agreed lobby. Its first version still hashed the local input device INDEX; that is fixed and asserted |
| `SessionScopedEntity` in the peer checksum | **CLOSED** (schema 184) — probed clone; still snapshotted, because the construction scope gather reads it |
| the peer-agreed match ordinal | **CLOSED** (schema 187) — `SessionMatchOrdinal` mints which match of the session it is; both the item draw context and `SimId::match_spawn` moved off the absolute activation tick |
| the four `MatchInstance`-stamped resources | **CLOSED** (schema 190), after being closed WRONG **THREE** times. 186 moved them onto the activation tick and called it peer-stable. 187's correction then excluded the instance ENTIRELY, which was false-NEGATIVE: a verdict for the previous match checksummed identically to one for the live match while `settled(active)` disagreed. 190 gave `MatchInstance` a LOCAL half (staleness, `belongs_to`) and a PEER half (the ordinal) — correct WITHIN one session, and the GPT review of 2026-09-16 found the third hole: the ordinal restarts at zero every session, so `session A / match 0` and `session B / match 0` project identically while the three stale-tolerant resources are App-global and can hold A's stamp beside B's first match. ⇒ Closed 2026-09-16 by making the ANTECEDENT impossible rather than the checksum longer: all four are members of `SessionScopedResources` now, reset at `SessionScopeSet::Activate`. Held by `a_match_stamp_from_the_previous_session_cannot_reach_the_next_ones_first_match` |
| `SessionMatchOrdinal`'s own registration | **CLOSED** (schema 189) — it was `rollback_resource_canonical`, whole-value, with a comment beside it claiming the `session` half "is compared only against ITSELF". The sentence described `take`; the registrar decided the checksum. ⚠ The lazy-reset window that was RECORDED here is closed as of 2026-09-16, by the same edge as the row above: the mint is reset eagerly at `SessionScopeSet::Activate`, so it cannot carry the previous session's count into this session's first activation. `take`'s own check survives as the answer for a composition that has only one session |
| `MatchInstance::random_context` | **CLOSED** — the method moved to `ActiveMatch` and reads the ordinal. This row said OPEN while the row above said CLOSED, which the review flagged as contradictory control-plane text |
| checkpoint operation keys | **CLOSED** (schema 188) — the peer projection is the ADMISSION SEQUENCE plus whether a scope owns the operation; the scope keeps its stale-operation job and still round-trips, because all three carriers snapshot by `Clone` |
| **the session root's canonical `SimId`** | **CLOSED 2026-09-16** — it was `SimId::singleton("session", activation_id)` on BOTH mints, and `ShellActivationId` is a per-App route count inside a `component-canonical` comparison. ⭐ The count was disambiguating NOTHING: a canonical identity only needs to be unique inside the world a checksum compares, and `shell_host_lifecycle` already pins `session_roots == 1` in game and `== 0` at home across a four-session lifecycle, rollback variant included. Both mints are `SimId::singleton("session", "root")`. Held by `two_hosts_with_different_route_histories_name_the_session_root_identically`. See below |
| `TransactionId` provenance | **CLOSED** (schema 193) — the campaign's original finding. The stamp still renders `{binding}\t{room}\t{session}` and MUST, because the construction scope's gather filter and A10's candidate-vs-live separation read it; the projection keeps the content identity and the room and drops the app-local epoch and the session stamp. It is the first COMPONENT to state a projection, which needed `rollback_component_canonical_checksum` to exist |
| the canonical timeline itself | ⛔ **OPEN, AND BLOCKED ON A MAINTAINER — `Q128`** in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). The absolute `SimTick` is `resource-canonical`, so two Apps running for different lengths of time disagree from the first compared frame. It cannot be closed the way the other nine were: a projection excluding the tick would exclude the TIMELINE, which is what a rollback comparison is about. It needs a session-relative tick rebased when peers agree to start, and where that agreement comes from is netcode. See below |

✔ **THE SESSION ROOT'S IDENTITY WAS A HOST-LOCAL ROUTE COUNTER, AND IS NOT NOW.**
Found by the GPT review of 2026-09-15, measured on the production road, and
closed 2026-09-16. The arm is
`two_hosts_with_different_route_histories_name_the_session_root_identically`
(`ambition_game_shell::session::tests`); it used to pin `session:11` against
`session:4` and now pins `session:root` against itself, with the two hosts still
activating different route counts so the premise stays non-vacuous.

⇒ **THIS IS A CLASS `id_peer_audit` STRUCTURALLY CANNOT GUARD.** That guard
censuses registered TYPE NAMES and `SimId` is a type that is supposed to be
canonical; the bad fact is its PROVENANCE. Two provenance defects have now been
found and neither was visible there — the match-spawn tick (inside a
constructor's argument) and this one (inside a singleton's key). Provenance is
held by value-level arms in the crate that MINTS the identity.

⭐⭐ **THE FIX WAS A CONSTANT, AND THE ARGUMENT IS THAT THE COUNT DISAMBIGUATED
NOTHING.** This section previously said the fix was *"NOT make it constant"* and
that an explicit `PeerSessionIdentity` was required. That was wrong, and the
measurement that settles it was already green and unread:
`shell_host_lifecycle`'s `assert_in_game` / `assert_home` pin
`session_roots == 1` and `== 0` at every point of a four-session lifecycle,
`the_full_multi_game_lifecycle_is_leak_free_under_rollback` included. A canonical
identity has to be unique inside the world ONE checksum compares, not across a
process's history.
`a_hidden_candidate_may_share_the_live_worlds_identity_and_a_published_one_may_not`
establishes that an A10 candidate root deliberately carries the SAME `SimId` as
the live root it replaces — a constant preserves that exactly — and an UNHIDDEN
duplicate is refused as `BaselineCaptureError::DuplicateIdentity`.

⚠ **AND THE RESIDENCY OBJECTION CUTS THE OTHER WAY.** The worry was that a
constant is unproved against future residency allowing two session worlds alive
at once. So was the activation count: a peer does not share your retirement
schedule, so a root that lingers on one host and not the other is ALREADY a
divergence whatever it is named. The invariant that would need restoring there is
*"exactly one session root is visible"*, not the identity.

⛔ **A PEER-STABLE `PeerSessionIdentity` IS STILL NOT DERIVABLE HERE, and that is
why it was the wrong road rather than merely an expensive one.** Two peers can
only agree on a session identity through something the session handshake carries,
and the only sessions in this repository are `SyncTestSession`. Anything minted
locally today would be a local value wearing a peer name — the disease, not the
cure. When real peers exist, the handshake is where it comes from.

⛔⛔ **`SimTick` IS AN ABSOLUTE PER-APP COUNTER AND IT IS ALREADY A WHOLE-VALUE
PEER CHECKSUM INPUT.** Measured 2026-09-15: one writer
(`ambition_time::advance_sim_tick`, `+1` per step), `init_resource`'d once at
App build, never rebased anywhere in the workspace, and registered
`resource-canonical`. It sits UNCONDITIONALLY at the head of the sim schedule,
so it counts menu frames. ⇒ **Two Apps that have been running for different
lengths of time disagree about `sim_tick` from the first compared frame**, before
anything else in this campaign matters. Everything keyed on it inherits that:
the first fix here moved four resources onto the activation tick and called it
peer-stable, which was the same error one layer down.

⚠ **`random_context` KEEPS the tick deliberately**, recorded rather than fixed.
Removing it with no replacement makes every match in a run replay the first
match's item drops — a visible regression pinned by
`two_activations_are_two_draw_contexts`. A checksum is compared every frame, so a
false desync there is fatal; a repeated item table is not. ⇒ **The replacement is
the match's ORDINAL WITHIN THE AGREED SESSION**: zero for everyone who joins
together, insensitive to menu time and prior sessions, and it still separates
consecutive matches.

⚠ **AND NOTHING IN THE REPOSITORY CAN CURRENTLY OBSERVE ANY OF THIS.** The only
sessions in use are `SyncTestSession` — one machine rewinding itself, zero
distance. A desync canary that compares a machine against its own past is
structurally incapable of catching a two-peer disagreement, which is why every
leak in the table above had to be found by reading.

**The standing guard, and what it could not see.** `id_peer_audit.rs` reads the
LIVE registry. ⛔ Until 2026-09-15 its population was a list of variant names
kept in the test, which omitted every `*CustomChecksum` kind. Measured against
the schema baseline: **143 registrations feed a peer checksum, the list named
114, and the 29 `*custom-checksum` rows were invisible** — the checkpoint family
among them. The question now lives on `RollbackEntryKind` itself as
`feeds_peer_checksum()`, where a new variant cannot be added without answering
it. Poison-verified in both directions: claiming a custom-checksum kind does not
feed the checksum reddens the guard and names what it stopped covering.

⚠ The review asked for this to be settled *before* A10 made transaction provenance
more central. That did not happen: A10's room scope landed first, and a publication
now declares the lane `TransactionId`s it owns (`PublicationEffects::owned_by`),
with `CandidateNotOwned` refusing anything stamped outside them. A10 deliberately
used the existing interfaces rather than hardening host-local lineage into a new
provenance contract, so the dependency stayed narrow — but it is wider than it was.
`a_transaction_stamp_depends_on_host_local_lineage_and_must_keep_doing_so`
(`shared_tangle/src/construction/tests.rs`) records the divergence and flips the
day the identities are split.

**What the acceptance has — BOTH AXES, as of 2026-09-16.** Two two-App witnesses
live in `game/ambition_app/tests/id_peer_audit.rs`, each building real Apps with
`build_visible_app` and each asserting its own premise before comparing anything.

1. `two_differently_aged_hosts_publish_the_same_roster_through_the_shipped_road`
   ages one host along the SHELL ACTIVATION axis by extra select-route visits,
   asserts the activation counters DIFFER, and asserts the shipped match-start
   road publishes the same roster from both.
2. `two_hosts_at_different_content_epochs_share_one_construction_provenance` is
   the CONSTRUCTION axis the review asked for and this paragraph used to record
   as unwritten. ⭐ The burn is the shipped road rather than a poked resource:
   content is prepared per load transaction and `prepare_platformer_content` ends
   `builder.finish(epochs.allocate(), ..)`, so entering a game, quitting to the
   title and entering again allocates a second `ContentEpoch` for byte-identical
   content. Observed epochs 3 against 1. It compares the `TransactionId` PEER
   PROJECTIONS, which must agree, while the local stamps still differ — the
   latter held by
   `a_transaction_stamp_depends_on_host_local_lineage_and_must_keep_doing_so`.

⚠ **THE SECOND ARM'S POPULATION IS ONE VALUE, AND IT SAYS SO.** The projection is
`binding ⊗ room`, so every entity constructed into one room shares a checksum; an
undeduped comparison prints eighteen identical numbers and reads like an
eighteen-wide population. It is deduped, and the anti-vacuity floor is written
against the deduped set. Found by reading what the POISON printed — the passing
run could not show it, because an `assert_eq` over two equal vectors says nothing
about their structure.

**Next implementation, in order.** (1) ✔ **DONE** — the peer-agreed match
ordinal; `ActiveMatch` carries which match of its session it is, minted by a
rollback-registered `SessionMatchOrdinal` that restarts at zero on a session
change. It closed `random_context` AND `SimId::match_spawn`, which was embedding
the absolute tick in a `component-canonical` identity string. Schema 187.
(2) The peer/local split on `CheckpointOperationKey` — the scope's
stale-operation job is local and real and must NOT simply be deleted;
`SessionCheckpointOperations` already advances its sequence only on ADMISSION for
exactly this reason. (3) `TransactionId` — see the measured shape below.
(4) The timeline itself — a session-relative tick, which is netcode work and
wants a maintainer decision before anyone starts.

⛔ **STEP 3 IS BLOCKED ON A VOCABULARY-PLACEMENT DECISION, MEASURED 2026-09-15 —
and it is NOT "drop the session term", which was this row's previous
instruction.** `TransactionId` is
`{epoch}\t{room}\t{session}` (`ConstructionScope::transaction`,
`crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs:716`) and is registered `component-canonical`, so the whole
string is compared between peers.

- **The session term must STAY.** Measured: the construction scope's gather
  filter is what isolates one session's entities from another's, so removing it
  would break A10's candidate-vs-live separation. This is the split the campaign
  header warns about — A10 needs exact LOCAL ownership, ID-PEER needs the token
  out of the peer COMPARISON. ⇒ The shape is `ActiveMatch`'s: keep the whole
  string, and give `TransactionId` a projected checksum that excludes the local
  terms.
- **A projection alone is not enough**, which is why the epoch has to move too.
  The only peer-stable term in the string today is `{room}`, and projecting to
  that would give every entity in a room one identity — a worse defect than the
  one being fixed. The projection needs a peer-stable CONTENT term to survive.
- ✔ **THE PLACEMENT DECISION IS MADE (2026-09-15) and the vocabulary is
  landed:** `ambition_platformer2d_core::PeerContentIdentity`, beside
  `ContentEpoch` in `content_epoch.rs`, as the PEER half of a pair whose LOCAL
  half was already there. ⭐ Decided from that module's OWN stated principle
  rather than by convenience: the epoch lives in the neutral foundation because
  *"several layers that must not name each other all need to state it"*, with
  preparation ALLOCATING and construction planning only STAMPING. The peer term
  has exactly that shape — planning must stamp WHICH CONTENT a plan was built
  against, and `ambition_platformer2d_runtime`'s content identity renders the
  value. Same split, same reason, same home.
  ⚠ Measured correction to this row's own earlier reasoning: it said the
  `content_pack` edge "is legal". It is not available —
  `ambition_content_pack` declares NO ambition dependencies at all, so it cannot
  construct a type from `platformer2d_core`, and `platformer2d_core` naming it
  would invert the graph.
  ⛔⛤ **AND A SECOND CORRECTION: THIS ROW NAMED THE WRONG
  `ContentFingerprint`.** There are TWO types with that spelling.
  `ambition_content_pack::prepared::ContentFingerprint` is `(pub u64)`;
  `ambition_platformer2d_runtime`'s is `digest_type!(ContentFingerprint, "cfp1:")`
  — a `[u8; 32]` with a private field. `PreparedContent::fingerprint()`, the
  accessor sitting beside `epoch()` and therefore the one any binding site can
  reach, returns the SECOND. The row named the first because a search for the
  name found the definition with the public field. ⇒ `PeerContentIdentity`
  carries 32 bytes, not a `u64`: folding a 256-bit digest into 64 is defensible
  for a checksum term and not for an identity string, and this is destined for
  both.
- ⚠ The plumbing is also real: the production binding site
  (`session/setup.rs`) receives `construction.binding` already built and never
  sees `PreparedContent`, so a fingerprint has to travel with the epoch from
  wherever content is prepared. 19 `ContentBinding::Content(` sites, against 45
  `.transaction(` callers — so the change belongs in what the BINDING renders,
  not in the transaction signature.
  ✔ **THE BINDING NOW CARRIES IT (2026-09-16).** `ContentBinding::Content` is a
  struct variant with `epoch` (local, staleness) and `content` (peer), all 19
  sites are migrated, and the two PRODUCTION sites are populated from
  `PreparedContent::fingerprint()` — which sits beside `epoch()`, so a site that
  can state the local generation can always state which content it is a
  generation OF. ⚠ `canonical_summary()` still renders the epoch ALONE, and an
  arm pinned that two bindings differing only in content summarised identically,
  so that the change could not happen silently.
  ✔ **AND THAT CHANGE HAS NOW BEEN MADE (schema 192).** `canonical_summary`
  renders `epoch:N|content:<64 hex>` when a content identity is STATED, so
  `TransactionId` carries a peer-stable term for the first time. ⚠ The segment is
  ABSENT rather than zero-filled when unstated, so a binding built outside a
  prepared session renders `epoch:N` exactly as before and every fixture's
  identity is byte-identical — only production strings moved.
  ⛔ **THE PROJECTION IS STILL NOT LANDED, and the reason is structural rather
  than pending work.** `TransactionId` is a bare `String` whose `from_raw` is the
  codec's decode half, so a checksum function — which receives only
  `&TransactionId` — must either PARSE the string or store a second field. The
  string cannot be parsed unambiguously: an unstated `epoch:4` and a
  `runtime-dynamic` binding both lack the `|content:` segment while meaning
  different things, and `from_raw` is called with synthetic values like
  `"t/candidate"` in tests, over which any parser returns something rather than
  refusing. ⇒ The honest shape is to restructure `TransactionId` into its parts
  with two renderings over one mint — the `MatchInstance` pattern — which is a
  61-use change and the next reviewable step. `ContentBinding::peer_stable_summary`
  is the term it will project, and it is landed and poisoned already.

⭐ **AND THE EPOCH'S OWN MODULE DOC ARGUED THE OPPOSITE UNTIL 2026-09-15.** It
said *"an epoch is not rollback-registered. Two peers never compare sequences, so
a gap on one host is invisible"* — the stated justification for letting a refused
reload BURN a number. The chain above is the refutation: not registered, compared
anyway, through whatever embeds it. Corrected at the definition.

⛔ Do NOT continue by mechanically replacing each raw `SessionScopeId` with the
nearest canonical-looking value. `SimTick` is why: it looks canonical, it
rewinds, it is already checksummed, and it is host-local.

**Acceptance:** two Apps that have burned different numbers of local session
activations can enter the same deterministic match and produce the same canonical
mechanical identity/checksum. The witness must first assert that their local
counters differ.

### ROLLBACK-KIND-SPELLING — one registration, one kind, spelled once

**Owner:** rollback registration (`platformer2d_runtime/src/rollback/registrar.rs`
and `platformer2d_rollback_ggrs/src/registration.rs`).

**Current state:** every registrar method spells its `RollbackEntryKind` TWICE —
once on the RECORDING road (`runtime`'s registrar, which writes the descriptor
the schema baseline and every census read) and once on the INSTALLING road
(`rollback_ggrs`, which adds the snapshot plugin and checksum system). Nothing
derives one from the other.

⛔ **MEASURED 2026-09-15, BY MAKING THE MISTAKE.** Splitting
`resource-canonical-custom-checksum` out of `resource-canonical`, I changed the
recording road only. The result was one registration arriving under two
different kinds, caught at app build by `RollbackRegistry`'s
conflicting-registration check — which is accidental cross-evidence, not a
designed guard, and **covers only names BOTH roads reach**. A kind spelled
wrongly on a registration that only one road installs has nothing checking it.

⭐⭐ **MEASURED 2026-09-16: THE TWO ROADS AGREE TODAY, AND THE DUPLICATION IS
STRUCTURAL RATHER THAN A DRIFT.** Every method name present on both roads spells
the SAME kind — zero disagreements. The roads are two SEPARATE TRAITS with
matching method names: `RollbackRegistrar` (declared in
`ambition_platformer2d_core::snapshot`, 24 methods, implemented by
`SchemaRollbackRegistrar` in `runtime` — 24, an exact match) and
`AmbitionRollbackApp` (declared and implemented for `App` in
`rollback_ggrs/src/registration.rs`). ⇒ There is no shared declaration to hang a
kind on, which is why "name it where the method is declared" needs the two
vocabularies collapsed first. That is the real shape of this row.

⚠ **AND A FIRST PASS OF MINE REPORTED FOUR RECORDING-ONLY METHODS THAT DO NOT
EXIST.** My script took the first `RollbackEntryKind::` after each `fn` and the
installing road nests differently, so it mis-grouped
`rollback_component_clone_checksum` and `rollback_resource_clone_checksum` and
their `_with_schema_detail` siblings. They are on both roads. The asymmetry is
ONE method, not five — see below. A parser's grouping is a finding about the
parser until it is checked by hand.

⇒ **ONE ASYMMETRY WAS REAL AND IS DELETED.** `rollback_resource_cursor` was
<!-- cite-ok: the deleted method is this paragraph's subject; a resolvable citation would mean the deletion did not happen -->
declared and implemented on the INSTALLING road only, with no counterpart in
`RollbackRegistrar` and ZERO callers in the workspace. A registration expressible
on one road and not the other is worse than a kind spelled twice: it installs
snapshot machinery the schema baseline has no row for, and the
conflicting-registration check cannot see a name only one road reaches.

**Next implementation:** collapse the two vocabularies before touching kinds.
Give each registrar method ONE kind, named where the method is declared rather
than at each call of `descriptor::<T>` / `record::<T>`. Do not add a third table
mapping method names to kinds; that is the same duplication with an extra hop.
⚠ `RollbackEntryKind` lives in `runtime::rollback::registry` and the shared trait
lives in `core::snapshot`, so the enum has to move before a default method body
can name it.

⛔⛤ **AND "DEMOTE THE GENUINELY-`derived` REGISTRATIONS" IS NOT A MECHANICAL
SUB-TASK — ITS FIRST NAMED EXAMPLE IS WRONG.** MEASURED 2026-09-16: of 212
distinct types registered through a `*_clone` method, 21 have a declaration doc
matching `derived|recomputed|never authored|never persisted`. Reading them, most
say "derived" about something ELSE — `ActorRenderSize`'s COLLISION BOX,
`CapturedBy`'s INVERSE, `AuthoredHurtboxes`' absence selecting a sprite-derived
box. ⚠ 21 is a floor of CANDIDATES, not a count of defects.

The ones that really do describe themselves that way are deliberate, and the
reason is written at the registration site
(`crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs:428`):
*"'Re-derived next tick' is not a reason to omit it: `ITEM 0` of this project's
own record is a component declared derived, dropped by a restore, and read before
its writer ran again. Presence is authoritative because a query FILTERS on it."*
`Dormant` and `SensesUndecided` are both registered for that reason;
`StrikeVolume` for a sibling one (without it "a rollback that rebuilds
`MovePlayback` from a blob would strand every live box forever").

⭐ **THE RULE: a component whose PRESENCE is read by a query filter is
AUTHORITATIVE even when its value is derived.** *"Derived"* describes how it is
COMPUTED; rollback cares whether anything READS it before its writer runs again.
The doc sentence that reads like a demotion candidate is usually answering the
first question. ⇒ The demotion population looks close to zero and nothing should
be demoted on a keyword match.

**Acceptance:** changing a method's kind in one place changes both roads, and a
poison that changes only one side fails to compile rather than relying on a
runtime conflict check. The existing conflict check stays — it covers a
different failure (two different registrations claiming one name).

### ROLLBACK-MUTATOR-POPULATION — the mutator guard sees a quarter of rollback state

**Owner:** rollback scheduling (`scripts/check_rollback_mutators_run_in_sim.py`).

**Current state:** the guard that keeps rollback state from being mutated outside
the rewinding schedule defines that state as `rollback_*_canonical::<T>` only.
Every clone and custom-checksum registration is outside its population —
`RoomSet` and `LdtkRuntimeIndex` among them — although those values are
snapshotted and restored on every rewind exactly like the canonical ones, so an
outside mutation drifts identically.

⛔ **MEASURED 2026-09-15.** Widening the population to all rollback registrations
takes the systems it can see from 555 to 564 and surfaces **65 unwaived
offenders**. ⚠ That number is not a defect count: many are `Transform` writes
from camera, sprite and inspection systems, which are PRESENTATION reading a
component that happens to be rollback-registered. The widening is not landable
until sim writes and presentation writes can be told apart.

⚠ A second, independent hole in the same guard was closed on 2026-09-15: its
param pattern matched only `&mut T` and `ResMut<T>`, so `SessionWorldMut<T>` was
invisible and a refactor that respelled one write made the guard report a live
mutation as gone. All six types reached through that param are rollback-registered.

⛔⛤ **A THIRD HOLE, CLOSED 2026-09-16, AND IT HID THE GUARD'S OWN FOUNDING
SUBJECT.** `#[derive(SystemParam)]` is a fourth spelling: a bundle is ONE
identifier in a signature and may hold any number of `ResMut` fields.
`SessionScopedResources` holds 25, **13 of them rollback-registered** —
including `MovingPlatformSet`, which is the single type this guard could see
before the 2026-09-02 widening. So it was blind to a mutation of the type it was
written around and reported clean. 60 such bundles exist across 42 files. ⚠ Half
the hole was the LIFETIME: a signature elides it (`ResMut<T>`), a struct field
cannot (`ResMut<'w, T>`), so the pattern was perfect on functions and matched
nothing in the bodies that mattered. Visible systems 333 → 349; findings 1 → 10,
each verified by reading rather than counted.

⭐ **THE GENERAL FORM, now that there are three: EVERY hole in this guard has
failed in the GREEN direction.** `&mut T` alone saw 1 type of 113,
`SessionWorldMut<T>` hid six, `SystemParam` hid thirteen — and each time the
report got SHORTER and CLEANER, which reads as good news. ⇒ `POPULATION_FLOOR`
now makes the population size part of the verdict, checked BEFORE the findings:
a fifth spelling cannot announce itself, but it cannot avoid making the count
fall. "No offenders" over a collapsed population was the one thing this guard
could never report.

⚠ **AND THE `handle_ldtk_hot_reload` ACCEPTANCE BELOW IS BLOCKED ON THE SAME
FACT AS THE WIDENING, which nothing had said.** MEASURED 2026-09-16: `RoomSet`
and `LdtkRuntimeIndex` are `rollback_component_clone_checksum`. Adding
`rollback_resource_clone[_checksum]` (113 → 139 types, 9 findings) does **not**
restore that waiver — only including COMPONENT clone registrations does, and
components are exactly where the 65 presentation `Transform` writes enter. So
the two acceptance clauses are one clause.

⇒ **THE RESOURCE HALF IS SEPARABLE AND IS NOT BLOCKED.** Resource clone
registrations bring no `Transform`, so widening to
`rollback_resource_clone[_checksum]` alone costs 9 findings rather than 65:
`reset_inventory_on_new_game` gains all four of its non-canonical facts
(`MintedItemBaseline`, `OwnedItems`, `OwnedItemsBaseline`, `SaveRestored`),
plus `adopt_occurrence_checkpoint_from_save`, `complete_durable_restore`,
`kaleidoscope_menu_action_activated`, `track_versus_roster`, and four
`AmbitionGameSave` persistence mirrors that likely want waivers rather than
fixes.

✅ **The resource half LANDED 2026-09-16** (`068bb6034`): population 113 → 139,
visible systems 349 → 398, findings 10 → 17. No waivers were invented —
`ambition_persistence/src/rollback_registration.rs` has said in prose since
2026-08-29 that *"the ~6 systems that pair a non-rewinding `Local` edge-detector
with these very resources would have failed SILENTLY once rollback went live"*,
and the four `AmbitionGameSave` writers ARE those systems, so waiving them would
re-hide the thing that comment warned about.

**Next implementation — triage the 14, which is owner work rather than guard
work.** Two are answered: the `reset_session_scoped_resources_on_*` pair took
explicit waivers in `869e3ee81`. My reading of the rest, offered so an owner
starts from a verdict rather than a list:

| finding | reading |
| --- | --- |
| `reset_inventory_on_new_game` | **REAL.** Producer `process_new_game_reset_request` runs in `sim_schedule()` under `ResetProcessing`; consumer runs in `Update`; `NewGameResetCommitted` is `clear_message_on_rollback`. Being moved onto its sibling `clear_transient_on_sandbox_reset`'s chain. |
| 7 menu systems → `NewGameResetRequested` | **REAL.** They carry `.run_if(simulation_authorized)`, so they write a `rollback_resource_canonical` resource with a live session. Reached through `MenuDispatchParams`, which is why they were invisible before `4f442eb11`. |
| 4 `AmbitionGameSave` writers | **REAL, AND A DIFFERENT FAILURE FROM THE ONE THE GUARD'S PROSE DESCRIBES.** That resource has a CHECKSUM projection, so two peers can disagree at ONE FRAME without anything drifting — the guard says "drifts a little further each time", which invites a reader to dismiss a one-frame disagreement. ⚠ `ambition_persistence/src/rollback_registration.rs` predicted exactly these in 2026-08-29: *"the ~6 systems that pair a non-rewinding `Local` edge-detector with these very resources would have failed SILENTLY once rollback went live."* |
| `adopt_occurrence_checkpoint_from_save`, `complete_durable_restore` | **UNRESOLVED, and the obvious argument does NOT transfer.** Both are one-shot latch-gated on `SaveRestored`, which looks like the activation waiver's "the write precedes the timeline". ⛔ But both also require a LIVE PRIMARY PLAYER BODY (`bodies.is_empty()` / `ready_body.single().is_err()`), and a live body means the session world root is live — which is the exact condition `maintain_local_session` gates GGRS start on. So they may run WITH a live timeline. They owe their own argument. |
| `track_versus_roster` | **UNTRIAGED.** Writes `VersusMatch` from the shell's versus setup. |

⚠ **The discriminator may not be "is there a rebase".** A New Game's
`NewGameResetCommitted` is produced inside the rewind window, is
`clear_message_on_rollback`, and is consumed in `Update` — which is broken
whether or not the room replacement rebases GGRS. If the real question is
"is the triggering MESSAGE cleared on rollback", then `SessionScopeActivated`
owes the same question and nobody has asked it.

**Separately, for the COMPONENT half:** give the guard a way to tell a
simulation write from a presentation write — most likely by schedule rather than
by type, since `Transform` is legitimately written in both.

**Acceptance:** the population is every rollback registration, not one
registration spelling; `handle_ldtk_hot_reload` is visible without its waiver
being deleted; a poison that respells a write in any supported param form still
reddens the guard; and the population floor fails when a spelling stops
matching.

### SETTINGS-ROLLBACK — finish the settings/mechanics admission boundary

**Owner:** rollback/mechanical-policy owners.

**Current state:** direct `UserSettings` reads have been removed from the
simulation schedule. Control-frame modes are projected per seat, and damage uses
`PlayerDamagePolicy`. The remaining damage policy is still forward-only across a
rollback timeline.

**Blocked by:** [Q127](awaiting-maintainer-decision.md#q127--are-difficulty-assist-and-player-damage-modifiers-match-wide-or-participant-specific).

**Next implementation:** after Q127, make the admitted damage policy follow the
chosen lifetime: match activation if match-wide, or deterministic per-seat input
if participant-specific. Do not reintroduce simulation reads of mutable
`UserSettings`.

**Acceptance:** rewinding/resimulating frame N observes the policy admitted for
that timeline, not whatever the settings UI contains now; the settings-to-policy
projection remains witnessed end to end.

### THROW-MODIFIERS — route throws through rage and staleness policy

**Owner:** Smash combat/knockback policy.

**Current state:** authored throw base/growth values reach launch, but the throw
road bypasses the rage and staleness modifiers used by ordinary strikes. This is
a mechanical consistency defect, not a request to retune all throws.

**Next implementation:** route throw launch through the same named modifier
policy where Smash semantics require it, then remeasure representative throws.
Keep authored throw formulas and move-specific values intact.

**Acceptance:** a controlled throw witness shows the intended rage/staleness
change, and a neutral arm proves base authored throw behavior is unchanged when
both modifiers are neutral.

### A2 — close the remaining projectile construction-identity hole

**Owner:** [`engine/projectile-contact-protocol.md`](engine/projectile-contact-protocol.md).

**Current state:** the swept-contact resolver, finite obstruction, exact ordering,
targeted delivery and compound solid-contact policy are established. Build-site
census coverage also exists.

✅ **THE PLAYER-CLONE ROAD IS CLOSED.** `spawn_requested_player_clone` built a
body with `BodyKinematics`, `PlayerEntity` and the full movement clusters, and
with no `SimId`, no `FeatureId` and deliberately no `PrimaryPlayer` — so
`ensure_sim_id` matched neither arm and skipped it on every tick, forever. The
site now mints `SimId::spawned(primary, counter.next())`, states
`SpawnOrigin::Dynamic`, and REFUSES to spawn when the primary has no identity to
descend from (ADR 0030). Guard:
`the_player_clone_road_builds_an_identified_body`
(`game/ambition_app/tests/player_clone_live.rs`).

⛔ **IT WAS INVISIBLE TO BOTH SHIPPED CENSUSES, AND THAT IS THE REUSABLE PART.**
`ensure_sim_id`'s `debug_assert` and `observe_damageable_body_identity` both
require `CenteredAabb` AND `ActorFaction` — `StrikeVictim`'s own pair. The clone
carries the box and no faction, so neither instrument could see it. ⇒ The
acceptance below said *damageable*, and the population the invariant needs is
`BodyKinematics`: a body can be simulated, and can desync, without ever being a
strike candidate. `UnmintedBodyCensus` is the instrument whose population is
right, and it is the one that named this body.

⚠ **`UnmintedBodyCensus` COUNTS OBSERVATIONS, NOT BODIES.** It judges every body
on every tick, so ONE unnameable body standing for fifty ticks reports ~50.
Measured `209 judged, 49 skipped` for a single clone; `0 skipped` after the
repair. Read it as observations or it sends the next reader hunting 49 bodies
that never existed.

**Remaining — the unnamed SPAWNER, one level above the projectile.**
`materialize_matching` (`ambition_projectiles/src/materialize.rs`) inserts no
identity and defers to `mint_spawned_sim_ids`; that is the designated late mint
for dynamic entities and is correct as a road. The hole is its input:
`deploy_sentry`, `open_vortex_well` and `open_temporary_gravity_well` each take
`id: Option<SimId>`, and each production caller computes it through a `match`
whose fallback arm is `_ => None`. An unnamed turret therefore spawns, and
`mint_spawned_sim_ids` then skips every bolt it fires — `sentry.rs` documents
exactly that chain break. ⚠ Reachable BY CONSTRUCTION; not observed in a shipped
run. ADR 0030 says such a site refuses rather than degrades.

**Next implementation:** turn that `_ => None` fallback into a refusal at those
three spawn owners, as the clone road now does. Do not add a fallback ID that
invents canonical identity from query order.

**Acceptance:** a MECHANICAL body — `BodyKinematics`, not merely a damageable one
— cannot reach the simulation unnameable; the witness names the construction road
and the body rather than reporting a population count.

⚠ **THE CLONE-ROAD RECEIPT ABOVE WAS MEASURED AT THE PRE-MERGE TREE `b9f2ece18`.**
At `ecbdf2297` no `app_it` test that steps the simulation terminates — the lane
ran in 3.53s before the merge and does not finish in 300s after it — so the
receipt stands on that tree and awaits re-measurement, rather than being a claim
about `main` today. Attribution is settled by matched probes (with and without
this work stashed, both hang identically), so the regression is in committed
`main`, not in the A2 repair.

### A12 — finish move-contact attribution and reflection identity

**Owner:** [`engine/authored-technique-admission.md`](engine/authored-technique-admission.md)
and combat/projectile occurrence identity.

**Current state:** ranged feedback carries `MoveOccurrence` end to end, so a
projectile launched by move A cannot be credited to whatever move happens to be
playing when it lands. Melee stamps use the same occurrence authority.

**Open engineering:** add the guard that a body which has started a move cannot
lose `MoveOccurrence` during ordinary body lifetime; finish reflection/contact
attribution after the product rule is settled.

**Blocked by:** [Q101](awaiting-maintainer-decision.md#q101--may-an-abilitys-own-contact-satisfy-the-launching-moves-connected-condition).

**Acceptance:** late projectile/melee feedback, reflection and independent
ability contacts cannot credit the wrong move occurrence, including across an
idle gap and rollback.

### A4 — separate control authority from body execution on the real schedule

**Owner:** accepted control writer map and actor-monolith frontier.

**Current state:** the prerequisite writer census is complete and did not find a
competing control authority. The old `PlatformerRuntimeSet` vocabulary is gone; <!-- cite-ok: the DELETED `PlatformerRuntimeSet` vocabulary, named on purpose: a resolvable citation here would mean the deletion did not happen -->
the real realization places body integration inside
`WorldPrepSet::Integrate`. Prior prose that mapped old and new set names by name
is not an implementation guide.

**Next implementation:** size the packet against the actual schedule seam:
control production, accepted control authority, then body execution/integration.
Keep the measured invariant that a body is advanced once per tick.

**Acceptance:** one accepted control fact feeds one body execution road; no
second body tick or hidden writer is introduced; schedule witnesses are placed
between actual neighboring phases rather than only `.after(...)` an abstract set.

## P1 — ownership, composition and iteration

### I2/I3 — finish independent content authoring and safe reload

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) and content
reload/preparation owners.

**Current state:** a prebuilt host can load edited move content without Cargo;
reload has explicit outcomes, no-op revisions do not advance generations, and
stale attempts carry the generation they were prepared against. The runtime
already consumes loadable content rather than requiring the legacy Rust move
tables.

**Open work:** converge the remaining reloadable registries on one explicit
prepare/admit/publish contract and settle the permanent authoring source.

**Blocked by:** [Q110](awaiting-maintainer-decision.md#q110--may-a-provider-keyed-fragment-registry-gain-a-named-hot-reload-replacement-operation)
and [Q104](awaiting-maintainer-decision.md#q104--is-the-rust-move-table-or-the-content-file-the-source-of-a-moveset).

**Acceptance:** changed content publishes exactly once under a new admitted
generation; identical content is a no-op; stale work refuses rather than folding
against a generation it did not read; no runtime road silently falls back to a
second authoring source.

### A9 — establish truthful minimal engine profiles

**Owner:** public SDK/composition architecture.

**Current state:** capability-footprint and absence-contract tooling can measure
what a profile actually links/installs. The remaining work is semantic: define
what each supported profile promises instead of optimizing for a crate-count
number.

**Blocked by:** [Q100](awaiting-maintainer-decision.md#q100--should-the-facade-pull-bevydebug-because-it-always-links-ambition_dev_tools),
[Q106](awaiting-maintainer-decision.md#q106--are-ambition_items-and-ambition_encounter-optional-facade-capabilities),
[Q108](awaiting-maintainer-decision.md#q108--which-capabilities-may-a-featureless-ambition_platformer2d-link),
and the admission policy in [Q97](awaiting-maintainer-decision.md#q97--may-authored-content-name-a-technique-this-composition-did-not-install).

**Next implementation:** encode supported profiles as named capability contracts,
then make construction/step witnesses and absence guards test those contracts.

**Acceptance:** each supported profile constructs and steps a real subject; its
promised absent capabilities are absent from installation and resolved dependency
closure; the full Ambition composition remains intact.

### A7 — make item occurrence authority explicit where it carries a real invariant

**Owner:** [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md)
and [`engine/item-writer-inventory.md`](engine/item-writer-inventory.md).

**Current state:** `GroundItem` construction is sealed, but that seal does not own
occurrence creation. Death drops, match spawns and other roads still mint
identity/provenance/custody facts at their occurrence sites. The earlier claim
that one component constructor created one occurrence authority was false.

**Next implementation:** centralize only the occurrence decisions that share an
actual invariant (identity, custody, provenance, rollback ownership). Do not add a
generic request bus merely to reduce writer count.

**Acceptance:** reward policy consumes accepted occurrence outcomes and cannot
become an alternative minting authority; every remaining occurrence creator has
an explicit ownership reason.

### BRAIN — finish truthful fighter attack selection

**Owner:** [`engine/fighter-brain.md`](engine/fighter-brain.md).

**Current state:** the truthful attack kit evaluates the action a press actually
produces, and the previous rung-9 quantization defect is closed. Current failures
are no longer evidence that the old attack-kit mapping is wrong. The remaining
work is the owner's F6 decision/menu problem: a brain must be able to stop or
change movement so a movement-incompatible authored move can become selectable.

**Next implementation:** complete the F6 menu/utility term on the owner plan.
Keep press generation separate from move utility; do not patch the evaluator with
a fighter-specific exception.

**Acceptance:** representative CPUs select movement-compatible and
movement-transition attacks from their authored menu across the intended
difficulty ladder, with no regression to the press/move identity contract.

### D-POTATO-ASPECT — finish low-tier sprite aspect/trim policy

**Owner:** [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).

**Current state:** systematic downscale/trim generation defects were repaired.
The remaining product choice is whether character sprites at `potato` may fall
back to the `0_25x` tier.

**Blocked by:** [Q69](awaiting-maintainer-decision.md#q69--at-potato-should-character-sprites-fall-back-to-the-0_25x-tier).

**Acceptance:** the same authored frame preserves the intended world-space trim
and aspect at each supported tier; missing tiers follow the explicit policy
rather than an incidental fallback.

### D72 — continue Smash parity from the inventory

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

**Current state:** the inventory is the source of feature/parity truth. Do not
turn this queue row back into a chronological parity diary.

**Next implementation:** take the next inventory row whose policy is settled,
implement it on the production path, update that inventory row and add the
production acceptance witness.

**Blocked where applicable by:** [Q62](awaiting-maintainer-decision.md#q62--keep-or-discard-the-epoch-captured-4741-line-mary_oldtk-delta),
[Q89](awaiting-maintainer-decision.md#q89--what-special-should-each-robot-stand-in-have),
[Q115](awaiting-maintainer-decision.md#q115--which-per-move-hitboxinflate-values-should-the-untuned-bone-derived-specs-carry),
and other product rows named by the inventory.

### D166 — make character authoring boundaries load-bearing

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

**Next implementation:** for each remaining duplicated authored/runtime value,
choose one authoring owner and make every runtime representation a projection or
admitted prepared value. Prefer deleting the second truth to synchronizing it.

**Acceptance:** the owner document can name one authoritative authored value for
each migrated fact, and production consumers cannot bypass its preparation or
projection boundary.

### D-SCENARIO-IDENTITY — confirm and then finish scenario cache identity

**Owner:** performance/scenario tooling.

**Current state:** current source inspection does not locate the named cache
subject in the tree. Treat that as an investigation requirement, not as
permission to implement an inferred replacement.

**Next implementation:** locate the current scenario cache/key owner and prove the
identity collision still exists. If the subject was removed or renamed and the
collision no longer exists, close this row. Otherwise include geometry identity
in the cache key at the owner boundary.

**Acceptance:** two scenario geometries with equal benchmark knobs cannot share a
cached result accidentally.

### DURABLE-HORIZON-CHECKSUM — the save mirrors write hashed state from `Update`

**Owner:** `ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs`.

**Current state:** five systems installed by `DurableSaveHorizonPlugin` sit in
top-level `Update` and mutate rollback-registered resources. MEASURED
2026-09-16, with each type's `RollbackEntryKind::feeds_peer_checksum`:

| system | writes | kind | hashed |
| --- | --- | --- | --- |
| `adopt_occurrence_checkpoint_from_save` | `CustodyBaseline`, `OccurrenceBaseline` | `ResourceCloneCustomChecksum` | **yes** |
| `complete_durable_restore` | `SaveRestored` | `ResourceClone` | no |
| `persist_inventory_to_save` | `AmbitionGameSave` | `ResourceCloneCustomChecksum` | **yes** |
| `persist_occurrence_horizon_to_save` | `AmbitionGameSave` | as above | **yes** |
| `persist_minted_item_horizon_to_save` | `AmbitionGameSave` | as above | **yes** |

⭐ **THE PLACEMENT IS DELIBERATE AND SAYS SO**, which is why this is a row and
not five waivers. `runtime/src/durable_save_horizon.rs` states it outright: "The
installed systems remain in top-level `Update`, outside rollback resimulation.
Their state is rewindable where required, but file/application side effects
themselves are not replayed as simulation ticks." That argument is sound for the
side effect — writing a file twice is not a desync.

⛔ **IT IS SILENT ON THE HALF THAT IS HASHED.** Four of the five write a value
that FEEDS THE PEER CHECKSUM. `AmbitionGameSave` is derived from simulation
state, so two peers in agreement derive the same bytes — but the derivation runs
in `Update`, which executes once per FRAME, while the value is snapshotted and
compared per TICK. A peer that rolled back and re-simulated three ticks ran the
sim three extra times and `Update` zero extra times; the other peer did neither.
⇒ The open question is whether the value hashed at a confirmed frame can differ
between a peer that rewound into it and one that did not. That is not answered
by "side effects are not replayed", and nothing in the tree answers it elsewhere.

⚠ **AND THE SIXTH SYSTEM ON THAT SAME `.chain()` ALREADY CARRIES A PARTIAL
WAIVER SAYING THE SAME THING.** `restore_inventory_from_save` is waived in
`check_rollback_mutators_run_in_sim.py` "FOR THE ACTIVATION CASE ONLY, AND THE
OTHER CASE IS OPEN", because `durable_horizon.rs` explicitly supports a
mid-session load. So the mid-session half of this question was already known to
be open for one member of the chain and was never asked of the other five.

⛔ **DO NOT INHERIT THE "BEFORE THE TIMELINE" ARGUMENT FROM THE SESSION-SCOPE
WAIVERS.** It was checked against these and it does NOT transfer:
`adopt_occurrence_checkpoint_from_save` and `complete_durable_restore` both
require a live primary player body, which is exactly the condition
`maintain_local_session` starts GGRS on. These run when a session can already be
live; the session-scope resets do not.

**Next implementation:** answer the per-frame-vs-per-tick question with a
sync-test, the way `rollback_full_reset.rs` answered its own — rewind across a
frame in which `persist_inventory_to_save` ran and compare the checksummed
value. ⚠ If it is clean, these are five waivers with a measurement behind them
and this row closes. If it is not, the fix is the shape `AmbientGravityRequest`
already uses: write a message, let the sim apply it. ⇒ Either way the guard
stays RED until somebody runs it — which is correct, and is why these were not
waived to make a count go down.

### MENU-RESET-MIDSESSION — the menu writes rollback state from `Update`

**Owner:** `game/ambition_app/src/menu` + `ambition_platformer2d_actor_monolith`.

**Current state:** `grid_menu_action_activated` and
`kaleidoscope_menu_action_activated` both write rollback-registered state from
`Update`, which does not rewind. Two types, one path:

  * `NewGameResetRequested` (`rollback_resource_canonical`) via
    `dispatch_menu_action` → `SystemMenuParams::request_reset`.
  * `OwnedItems` via `dispatch_menu_action` → `dispatch_item_confirm`, which is
    what an equip or a consumable use goes through.

⭐ **THE TWO TYPES FAIL DIFFERENTLY, AND THE LOUDER ONE IS THE LUCKIER ONE.**
Both are written from a LOCAL menu, so only one peer makes the write; what
happens next depends on the registration kind, which
`RollbackEntryKind::feeds_peer_checksum` decides.

| type | kind | feeds the peer checksum | so a local menu write |
| --- | --- | --- | --- |
| `NewGameResetRequested` | `ResourceCanonical` | **yes** | makes A's and B's checksums differ — a DETECTED desync |
| `OwnedItems` | `ResourceClone` | **no** | is restored away on the next rewind, silently |

⛔ **SO `OwnedItems` IS THE ONE TO WORRY ABOUT.** Its kind is documented as
"snapshotted but not hashed: a rewind restores them, no peer reads them", and
that is exactly why nothing would report it: the player equips an item, a
rollback restores the pre-equip value, and the item is simply back in the bag
with no error anywhere. ⚠ `OwnedItemsBaseline` IS registered
`rollback_resource_clone_checksum`, so a projection of this state is hashed —
whether that projection would catch this write is the question to settle, not an
assumption to inherit from the kind's reassuring detail string.

⚠ **AND THE EXISTING TEST DOES NOT COVER IT, DELIBERATELY.**
`game/ambition_app/tests/rollback_full_reset.rs` asks whether the reset
RECONSTRUCTION is rollback-safe, and its own header says it folds a pending
request "into the baseline" so the work runs on the baseline frame and every
re-simulation of it. That is the safe shape by construction: a flag already true
before the sync-test window opens is identical on every peer and on every
replay. The mid-window menu write is the case nobody has asked about.

**Next implementation:** answer the narrow question first — can a menu that
writes these be open while a GGRS session is live? If it cannot, this is two
waivers with that citation and nothing else is owed. ⛔ Do NOT answer it from
the menu's own state machine; answer it from what gates the menu, because "you
would not do that" is not a property of the code. If it CAN, the write belongs
behind a message the sim consumes, the way `AmbientGravityRequest` already does
it for `BaseGravity` — that pattern is three lines away in the same bundle
(`gravity_requests`, with the comment "the sim applies the request").

Found 2026-09-16 by `scripts/check_rollback_mutators_run_in_sim.py`. ⚠ Five
sibling menu systems were flagged with these and are WAIVED, not fixed: they
take the same `SystemMenuParams` bundle and never reach `request_reset`. If the
bundle is ever split so access matches use, drop those five waivers — they exist
only because the bundle over-grants.

### ORPHAN-ARMS — 36 test arms that no `mod` line compiles

**Owner:** `ambition_boss_encounter`.

**Current state:** `crates/ambition_boss_encounter/src/pattern/tests.rs` holds 36
`#[test]` arms and is declared by nothing. `pattern/mod.rs` names six child
modules and `tests` is not among them; the three sibling `mod tests;` lines in
that directory (`control_flow.rs`, `content_schema.rs`, `validator.rs`) each
pull in their OWN `tests` subdirectory. So the file has never compiled and its
arms have never run.

⛔ **THIS IS THE GREEN-REPORT FAILURE IN ITS PUREST FORM.** The arms exist, they
are written, they are named after real claims, and `cargo test` reports every
one of them as neither passed nor failed because it has never heard of them. A
reader counting test files sees coverage that does not exist.

Found 2026-09-16 while resolving every name-excluded file to its declaration
(see `GUARD-CORPUS`). Pinned in `scripts/tests/test_test_paths.py` so the
count cannot grow quietly, which is NOT the same as fixed.

**Next implementation:** decide whether the arms still state something true of
`pattern/`, then either declare the module (`#[cfg(test)] mod tests;` in
`pattern/mod.rs`) and fix whatever fails, or delete the file. ⚠ Do not declare
it and then waive the failures — arms that never ran have never been green, so
a first run is evidence, not a regression. Drop the entry from the arm's
`orphans` list in the same commit.

### GUARD-CORPUS — five copies of "what is a test file", drifted

**Owner:** repo tooling (`scripts/check_*.py`).

**State: CLOSED 2026-09-16.** `scripts/lib/test_paths.py` owns the rule and all
five call sites use it. Floors landed first, in `993fdef58`; the consolidation
followed in `e660c2fc4`.

⚠ **TWO SESSIONS IMPLEMENTED THIS ROW AT THE SAME TIME AND NEITHER KNEW.** The
work was done twice, independently, down to the same four measurements — one as
`lib/test_paths.py`, one as `lib/rust_sources.py`.
<!-- cite-ok: the deleted duplicate is this sentence's subject; a resolvable citation would mean it was never deleted -->
The duplicate was deleted and its two non-overlapping pieces folded in: a brace-depth guard on the
`#![cfg(test)]` match, and `scripts/tests/test_test_paths.py`. ⇒ A row marked
with an owner and a "next implementation" still says nothing about whether
somebody is in it RIGHT NOW. Say so in the row, or in a message, before starting
a step that takes hours.

The five copies had drifted into five answers. `*_tests.rs` reached two of five
even though the docstring that added it records missing "all 51 of them", and
the population of copies was never enumerated — only the one somebody was
looking at — which is why that fix stopped where it did.

⭐ **WHAT THE CONSOLIDATION ACTUALLY BOUGHT, which was not the deduplication.**
Every copy tested a NAME. What removes a file from a build is `#[cfg(test)]` on
its `mod` line — which lives in the PARENT, not the file — or `#![cfg(test)]`
at the top of the file. Resolving all 284 name-excluded files to their
declaration found 281 genuinely gated, **two that shipped**, and one declared by
nothing at all:

| finding | file | disposition |
| --- | --- | --- |
| `mod tests;` and `pub(crate) mod test_support;`, both ungated | `enemy_projectile/mod.rs` | gated in `7301157d0`; the module doc comment already called it "test-only" |
| declared by no `mod` line in its crate | `boss_encounter/src/pattern/tests.rs` | **open** — 36 arms that never compile and never run |

⛔ **SO FOUR GATES HAD BEEN DROPPING SHIPPING CODE FROM THEIR CORPORA BY NAME,
AND EVERY ONE OF THEM REPORTED CLEANER FOR IT.** That is the failure this row
predicted, found in the direction it predicted. `scripts/tests/test_test_paths.py`
re-runs the comparison, so a name is trusted only for as long as it stays true;
a second arm pins `feature = "test-support"` to `[dev-dependencies]`, since
`#[cfg(any(test, feature = ...))]` is the one predicate the file rule cannot
settle alone.

**The ordering held up under measurement.** Each adoption was run before and
after:

| check | effect of adopting the owner | floor |
| --- | --- | --- |
| `check_rollback_mutators_run_in_sim.py` | verdict identical; already the widest of the five | untouched |
| `check_set_pins_have_engine_members.py` | 1367 → 1293 files, 225 → 222 pins | **CAUGHT IT**; lowered in the same commit |
| `check_capability_ships.py` | 1334 → 1264 production files, 414 → 408 writer types | held |
| `test_every_smash_technique_has_a_translator.py` | 282 → 257 ruleset files | **CAUGHT IT**; lowered |
| `ecs_inventory.py` | 3 module summaries, 7 fixture spawn sites | held |

⇒ Two of five floors fired on a change their author believed was safe, and both
times the drop was legitimate and had to be argued for in writing before the
floor moved. ⛔ **A FLOOR MAY GO DOWN ONLY IN THE COMMIT THAT CAUSES THE DROP,
AND ONLY WITH THE FILES THAT LEFT NAMED.**

⚠ **A claim made here and withdrawn.** I first recorded that all four
`test_support.rs` files carry `#![cfg(test)]`, so the name half was subsumed by
the attribute half — from misreading my own bucketed output. NONE carries it,
and two had no gate anywhere. The arm exists because that is exactly what a
comment cannot hold.

⚠ **Still one rule per reader, not one rule.** About 25 further inline copies
(`"/tests/" in path`, `endswith("tests.rs")`) live in reporting scripts that
gate nothing. They were left alone: a wrong exclusion in a report is visible to
its reader, and giving them a floor first is the same ordering all over again.

⛔⛤ **AND THE FLOOR HAS TO LIVE INSIDE EACH SCRIPT — AN EXTERNAL SWEEP CANNOT
SUBSTITUTE FOR IT. MEASURED 2026-09-16 BY FAILING TO DO EXACTLY THAT.** I tried
to answer "which of the 29 `check_*.py` pass over an empty tree" by importing
each, patching its `REPO`/`ROOT` global to an empty directory, and calling
`main()`. First result: **14 of 29 passed.** It was an ARTIFACT.

⇒ These scripts spell their entry points `def collect(repo: Path = REPO)`. A
default argument binds at DEFINITION time, so patching the module global
afterwards is inert — the script re-scans the real tree and passes, which is
indistinguishable from passing vacuously. Adding one control to the probe (the
empty-tree output must DIFFER from the real-tree output, or the redirect did not
take) cut the answerable population to 7 of 29, all of which correctly refused.

⭐ **SO THE HONEST STATE IS "NOT MEASURED", NOT "14 ARE VACUOUS".** The number
looked like a finding, had a mechanism, and was wrong. ⚠ The 22 unanswerable
scripts are unanswerable BY THIS PROBE and nothing is implied about them either
way.

⇒ **WHICH IS THE ARGUMENT FOR THE ORDERING ABOVE RATHER THAN AGAINST IT.** A
population floor asserted inside the script (`POPULATION_FLOOR` in
`check_rollback_mutators_run_in_sim.py`) needs no redirect, no import surgery
and no probe: it fails when the script's own reach falls, on the real tree, in
the lane that actually runs. That is the only form that survives a consolidation
which makes every check see less.

⇒ **STEP ONE IS DONE: ALL FIVE NOW HAVE A POPULATION FLOOR (2026-09-16).**
`check_rollback_mutators_run_in_sim.py` and `check_set_pins_have_engine_members.py`
already had one. Added to the other three, each poisoned by widening its own test
exclusion until the reach really fell:

| script | floor term(s) | measured | poison |
| --- | --- | ---: | --- |
| `check_capability_ships.py` | files scanned / production files / optional-read types / writer types | 1546 / 1334 / 191 / 414 | RED |
| `ecs_inventory.py` | crates / components / resources / registered systems | 78 / 638 / 493 / 1108 | refuses, 0 writes |
| `test_every_smash_technique_has_a_translator.py` | ruleset files in the haystack | 282 | RED |

⚠ **AND `test_every_smash_technique…`'s RISK RUNS THE OTHER WAY.** For the other
four a WIDENED exclusion is the silent danger — they report cleaner when they see
less. There, over-exclusion shrinks the HAYSTACK and produces MORE orphans, so it
fails loudly. Its silent direction is an exclusion too NARROW, letting a test file
into the haystack so a const named only by a test reads as a connected technique
— which is the recorded defect in its own docstring. ⇒ When the five collapse
onto one owner, that call site must not LOSE exclusions, and no floor can see
that. The floor makes the consolidation reviewable, not safe.

⚠ **AND `check_capability_ships.py`'s FLOOR IS A PEER'S, NOT MINE.** We wrote one
each in the same hour and theirs is better decomposed: it separates *files
scanned* from *production files*, so a widened test exclusion shows up as the
production count falling DIRECTLY rather than as a side effect on the type
counts. Mine was dropped rather than merged — two floors on one script is the
disease this row is about. Re-poisoned after taking theirs: widening `_is_test`
to swallow every `.rs` file still fails it.

⛔⛤ **AND `ecs_inventory.py`'s FLOOR WAS WRONG ON ITS FIRST WRITING — ITS OWN
POISON FOUND IT.** The check sat after the per-crate loop had already written its
shards, so a refused run left `.agent/ecs_inventory/crates/` shrunken beside a
stale `project.json`: a PARTIALLY PUBLISHED inventory, worse than the shrunken one
it was refusing. Now nothing is written until the floor passes — measured, the
poisoned run writes 0 files where the good run writes 162.

⇒ **STEP TWO IS DONE: ONE OWNER, `scripts/lib/test_paths.py` (2026-09-16).** All
five call sites delegate to `is_test_path`, which is the UNION of the five name
rules plus the inner `#![cfg(test)]` fact none of them checked. The local names
stay; only the rule moved.

⭐ **AND THE FLOORS EARNED THEIR ORDER IMMEDIATELY — TWO OF THE FIVE WENT RED ON
THE REPOINT**, which is exactly the review this row said a consolidation could
not otherwise get:

| script | reach before → after | verdict |
| --- | --- | --- |
| `check_capability_ships.py` | production files 1334 → 1264 | passed; floor 1250 held |
| `check_set_pins_have_engine_members.py` | sources 1367 → 1293 | **RED**, floor lowered 1300 → 1280 with the reason |
| `ecs_inventory.py` | 8 files newly excluded, 0 regressions | passed |
| `test_every_smash…` | ruleset files 282 → 257 | **RED**, floor lowered 270 → 250 |
| `check_rollback_mutators_run_in_sim.py` | unchanged (already the widest) | 14 findings before and after |

Each drop was checked rather than waved through: all 74 files the set-pins check
newly excludes are `*_tests.rs` it had never matched, plus the four whose inner
`#![cfg(test)]` compiles them out. `sets pinned` fell 225 → 222 — three pins that
lived in test files and were never production pins.

⭐ **THE INTERESTING RESULT IS A NEGATIVE ONE.** `test_every_smash_technique…`
still passes over its smaller haystack, so no authored technique was being kept
"connected" by a mention in a test file. That guard is strictly stronger now and
found nothing — worth recording, because a silent widening would have left nobody
able to say so.

⚠ **AND MY `ecs_inventory` FLOOR WAS CALIBRATED AGAINST THE WRONG READING.** I
took 638/493/1108 from the architecture census's `generated_inventory_counts`,
which is its snapshot of a PREVIOUS `.agent` generation, and set floors under
numbers this scanner does not produce today (659/528/1095). They PASSED, which is
why it would not have announced itself — a floor under a stale reading is in the
wrong place, not broken. Re-baselined against the scanner's own output.

**Acceptance:** one owner for "is this file test-only", covering both the name
conventions and `#![cfg(test)]`; each consuming check fails when its population
falls; and no check reports a `#![cfg(test)]` file's registrations as
production. ⇒ **MET 2026-09-16**, except that the fifth clause is only known for
the four files carrying an inner `#![cfg(test)]` today; a new one is covered by
construction rather than by a test.

### TEST-LANES — keep required test lanes executable and diagnose `app_it` flake

**Owner:** test runner / app integration lane.

**Current state:** missing prerequisites are reported as incomplete rather than
pass. The `app_it` lane RUNS AGAIN — 676 passed / 0 failed / 25 ignored of 701
at `23f786757`, 258.28 s — after the sim-schedule cycle below was closed. The
A10 work also observed one non-reproducing session-root handoff failure whose
assertion message was not captured. ⚠ The composition probes still do not step
the engine; see the next implementation step.

⛔⛤ **THE ORDER-DEPENDENT FAILURE HAS A NAME AND A MECHANISM NOW, AND MY FIRST
CLASSIFICATION OF IT WAS WRONG.** The arm is
`composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`.
I first recorded it as machine CONTENTION, because it appeared while two or three
full suites were running concurrently — then it reproduced **serially, on a quiet
machine, with one rust process and a load average of 2**. A coherent measured
story that fits the first observation is still the wrong one if it was never
tested against a second.

MEASURED at `770ac4bff`:
- Intermittent across runs of the FULL suite: 674/1 and 675/0 on the same tree.
- **12 of 12 green running that test file ALONE**, so it needs the whole process.
- The failure is a PANIC inside `FixedMain`, not a timeout and not a kill —
  `Encountered a panic in system bevy_app::main_schedule::FixedMain::run_fixed_main`.
- ⚠ The panic's own message is SWALLOWED: the arm has no assertion (it builds the
  engine with and without one plugin and steps 8 frames), and libtest's capture
  shows only bevy's three "Encountered a panic in system" lines. That is why this
  has read as a silent flake for so long.

⛔⛤ **AND THE PROCESS-GLOBAL HYPOTHESIS WAS ALSO WRONG. THE CAUSE IS TWO REAL
DEFECTS, BOTH FIXED 2026-09-15.** A source review found them without another
suite run; the "needs the whole process" reading was a timing artefact, not
shared state.

1. **A CAPABILITY LEAK.** `drive_boss_animators` takes `Res<BossCatalog>`, whose
   sole production initializer is `BossEncounterSimulationPlugin`
   (`boss_encounter/src/lib.rs`). `WorldPrepSchedulePlugin` registered it with
   `.after`/`.before` edges and **no `.in_set(...)` at all**, while every sibling
   boss system is `.in_set(WorldPrep)` — nested under `GameplaySimulationRoot`
   and its `simulation_authorized` gate. So it ran in a composition whose own
   capability was disabled. ⇒ Same family as
   [[reference_moving_systems_out_of_a_plugin_drops_their_set_membership]]: the
   ordering edges were remembered and the set membership was not.

2. **THE PROBE STEPPED NOTHING.** `the_engine_steps_with_and_without` builds the
   engine and calls `app.update()` eight times, but `add_headless_foundation`
   brings `MinimalPlugins`, which leaves `TimeUpdateStrategy::Automatic` in
   force — so `run_fixed_main_schedule` executes `FixedUpdate` **zero or more**
   times depending on elapsed WALL TIME. A fast run never crossed 1/60s and the
   arm reported success having exercised nothing.

⇒ **(2) IS WHY (1) READ AS A 50/50 FLAKE.** Pinning
`TimeUpdateStrategy::ManualDuration(1/60)` turned an intermittent silent kill
into a deterministic 1.58-second failure naming its own cause:

    Encountered an error in system `drive_boss_animators`:
    Parameter `Res<'_, BossCatalog>` failed validation: Resource does not exist

⚠ **AND NOTHING WAS SWALLOWING THE PANIC.** The message was always there; it is
printed on a Bevy task-pool worker thread, and only the nested
`Encountered a panic in system ...` propagation banners reached libtest's
per-test capture. There is no panic hook to fix — the arm had no assertion, so
the banners were all it showed.

⛔⛔ **AND FIXING (2) UNCOVERED A THIRD DEFECT THAT WAS WORSE THAN BOTH.
CLOSED 2026-09-16 at `23f786757`: IT WAS A DEPENDENCY CYCLE IN THE SIM
SCHEDULE, AND IT WAS NOT ABOUT TIMING AT ALL.** `b9f2ece18` gave
`drive_boss_animators` `.in_set(WorldPrep)` to close (1). That system also runs
`.after(project_boss_attack_state_from_move)`, which is
`.in_set(CombatSet::Playback)` — LATER in the sim schedule than `WorldPrep` — so
the set ordered one system both before and after Playback. The fix takes the
GATE and not a phase: `GameplaySimulationRoot` is configured
`run_if(simulation_authorized)` and pins no position.

BISECTED over `770ac4bff..ee3d0852e`, every point re-measured on
`a_dropped_item_falls`: `770ac4bff` and `91f0721bd` 3 passed in ~1.7 s;
`bad9ca8153`, `ecbdf22971` and `b9f2ece18` HANG; HEAD with the `.in_set`
replaced by the gate, 3 passed in 1.63 s. Lanes green at `23f786757`:
`app_it` **676 passed / 0 failed / 25 ignored / 701 total, 258.28 s**, and
`-p ambition_platformer2d_runtime --lib` **64 passed / 0 failed, 0.13 s** — two
arms of that lane had been hanging for a peer, on a lane the fix was not
measured against.

⛔⛤ **THE STANDING PROHIBITION, which is the INVERSE of the rule that produced
it.** `b9f2ece18` was written against *"carving systems out of a plugin drops
their set membership"* — edges remembered, set forgotten. ADDING a set to a
system that already carries cross-phase ordering edges fails the same way:
`WorldPrep` carries a POSITION as well as a gate, and only the gate was wanted.
⇒ Before adding `.in_set(X)`, check which set every existing `.after`/`.before`
target lives in.

⛔⛤ **AND THE LANE THAT WOULD HAVE CAUGHT IT WAS SKIPPED BECAUSE OF THE DEFECT
ITSELF.** `b9f2ece18` shipped with `cargo check` only and an explicit *"NO
`app_it` run: the composition probes can run the box out of memory, and the one
arm this change affects does not execute the simulation at all"*. Both clauses
were true. A schedule cycle is invisible to `cargo check` and invisible to every
arm that never steps, so those two facts were exactly what made `app_it` the
only instrument that could see it. ⇒ See
[[reference_a_gate_lane_you_did_not_run_is_a_guard_that_does_not_exist]].

⛔⛤ **AND (1) WAS NOT CLOSED BY THE GATE. A SESSION GATE IS NOT A CAPABILITY
CHECK.** `23f786757` argued `.in_set(GameplaySimulationRoot)` closes the
`Res<BossCatalog>` leak because the set carries `simulation_authorized`.
MEASURED IN ISOLATION that looked right — dropping the set failed exactly
`a_host_that_omits_boss_encounters_still_builds_and_steps`, 5 passed 1 failed.
⇒ It was wrong. Once the probes really stepped, the FULL lane failed that same
arm WITH the gate in place (674/2 at `1ac88c713`). `simulation_authorized`
answers *"is this session authorized"*, which is TRUE in hosts that have no
catalog. CLOSED at `582186bff` by asking the capability's own question:
`.run_if(resource_exists::<BossCatalog>)`. The set stays only because it pins no
position.

⚠ **A POISON MEASURED IN ISOLATION CERTIFIED A CLAIM THE FULL LANE REFUSED** —
same arm, opposite verdict. An arm that needs the whole process to fail cannot
be poison-verified alone.

**DONE (was the next implementation step).** The composition probes really step
as of `582186bff`. `step_the_fixed_schedule` pins
`TimeUpdateStrategy::ManualDuration(1/60)` AND asserts a `FixedUpdate` counter
is non-zero — the pin alone is not enough, because anything that stops the fixed
loop advancing returns these arms to certifying a build and that failure is
SILENCE, not a red. POISONED: removing the pin fails all three arms with the
counter's own message, exit 101.

**Current lane state.** `cargo test -p ambition_app --test app_it` →
**675 passed / 1 failed / 25 ignored of 701, 252.02 s** at `582186bff`. ⛔ The
one red is ID-PEER's, not this row's:
`an_edit_reaches_the_shipped_game::a_committed_world_reload_applies_its_effects`
fails on `ContentBindingMismatch` with the LIVE `PeerContentIdentity` reading
thirty-two ZERO bytes against a populated planned one, epochs equal. Green in
the full run at `23f786757`, red after the merge bringing `9e222ffb2`. Reported
to its owner.

⛔ **OPERATIONAL RESIDUE, kept because it cost a night.** An affected arm
allocated without bound and SUPERLINEARLY — ~27 MB/s over the first 60 s, then
~234 MB/s; one left running reached anon-rss 64,629,160 kB in 316 s and took a
62 GB box down with a kernel `global_oom`. ⇒ Any instrument that samples 60 s
and fits a line under-reports by an order of magnitude.
`scripts/measure_test_arm_rss.py` bounds this: one process per arm (peak RSS is
a property of a PROCESS, so the only way to make it a property of an ARM is to
stop sharing), `RssAnon` rather than `VmRSS` or cgroup `memory.current`, kill by
process group at a hard cap, and it refuses a row where libtest ran zero tests.
⛔⛔ **AND `pkill -f <pattern>` IS NOT A SAFE CLEANUP.** The shell running it is
a `bash -c '<whole line>'`, so its OWN argv contains the pattern and the first
`pkill` kills the shell — the second one, aimed at the test binary, never runs,
and neither does the verifying `pgrep`. That is how a 61.6 GB orphan escaped a
sampler whose cap was working on every other arm. ⇒ `pgrep -af` to LIST, kill by
PID, re-`pgrep` in a SEPARATE tool call.

**Next implementation:** on the next reproduction, capture the full failing
assertion and isolate the production ordering/state source before changing test
ordering or adding retries. Keep compile-cost and prerequisite failures distinct
from behavioral flakes, and from CONTENTION.

⇒ **THE `BodyWallet` RED IS CLOSED AT `4ccfef59c`, AND IT WAS A CROSSING RATHER
THAN A SCHEDULE.** `check_rollback_mutators_run_in_sim` reported
`reset_inventory_on_new_game` mutating `BodyWallet` from `Update`. The row had it
as a choice between two costly remedies; the PRODUCER decided it.
`process_new_game_reset_request` runs in the SIM schedule's `ResetProcessing`,
and `NewGameResetCommitted` IS `clear_message_on_rollback`. So the message was
produced inside the rewind window and consumed outside it, and a rewind could
clear the trigger before its consumer ran while the rollback-registered writes
that consumer had already made stood. ⇒ A waiver was never available.

The fix is a MOVE onto a road that already existed:
`clear_transient_on_sandbox_reset` consumes that same message in the sim
schedule, chained after the producer so its deferred `world.write_message` has
flushed. `reset_inventory_on_new_game` now runs `.after` it.

⭐ **AND IT ANSWERS THIS ROW'S OWN CAVEAT BETTER THAN THE CAVEAT EXPECTED.** The
row worried the ordering survives "on frames that HAVE a fixed step". The save
WIPE is inside the producer's queued closure, which is also in the sim schedule —
so wipe and reset now stand or fall together on the same frame, and the
"old run's bag into the freshly wiped save" hazard cannot open a frame-shaped gap
at all. Previously the wipe was in sim and the reset in `Update`, which is where
that gap lived.

⭐⭐ **THE REUSABLE PART: A RED HERE HAS THREE INDEPENDENT QUESTIONS BEHIND IT,
and answering one is not a verdict.** (1) is the write inside the rewind window;
(2) is the write at a point no rewind can CROSS, which satisfies the guard
without moving anything; (3) is the TRIGGER erasable by a rollback, which is what
closes the WAIVER route and which (2) cannot rescue. ⚠ MEASURED while handing
this to a peer: `SessionScopeActivated` is produced AND consumed in `Update` and
is NOT `clear_message_on_rollback`, so (3) does not touch the session-activation
reset — but that road still owes (2) for its sixteen rollback-registered
resources. Reporting (3) as the verdict would have told a peer they were clear of
a charge they had not answered.

⚠ **NINE FINDINGS REMAIN AND NONE ARE THIS ONE** — verified against the WIDENED
guard a peer landed while this was in flight, not the version that reported it.
Seven menu systems reach `ResMut<NewGameResetRequested>` through
`MenuDispatchParams`; two are session-reset roads. All separately owned.

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.

## P2 — product/authoring work with an executable owner

- **Character feel / Smash tuning:** use the real roster and the measurement tools
  named by [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).
  Do not infer the roster from `game/ambition_content/src/*_moveset.rs` or from one
  demo registration table. Product values waiting on a ruling stay in the
  decision ledger.
- **Q80 art/hitbox tolerance:** once the pixel tolerance is chosen, encode it in
  authoring/tool validation rather than subjective screenshots.
- **Q94 residency target:** once the memory target is chosen, use the asset owner
  plan to trade tiers/residency against a measured budget.

## P3 — human-gated or local-machine measurements

Do these only on a machine/environment that can answer the question:

- **D-RASTER-3:** measure weak-GPU framebuffer scale versus source-tier behavior.
- **Switch Pro outer range:** run the controller diagnostic on both target
  machines and compare the raw range.
- **Web reveal branch:** validate the existing reveal-barrier branch in the real
  browser/runtime.
- **Kaleidoscope Bevy-0.19 flash:** reproduce interactively before filing a fix.
- **LDtk preview tilesets:** measure whether editor-preview assets are still
  required by the authoring workflow before Q82 is resolved.
- **Capture after window close:** reproduce against the current capture path.
- **External consumer/platform checks:** follow the SDK/external-consumer owner
  documents; do not infer support from workspace-only builds.

## Replenishment rule

Before adding or promoting a row:

1. inspect current HEAD and confirm the problem still exists;
2. link the focused owner document;
3. state current behavior, next implementation, blockers and acceptance;
4. create/name a `Q` for every maintainer decision that blocks the row;
5. keep measurements in the owner document or a durable receipt, not as queue
   chronology;
6. remove closed rows instead of preserving their investigation history here.
