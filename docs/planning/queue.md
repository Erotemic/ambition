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

**Current state (2026-09-16): NINE CLOSED, AND AN ELEVENTH ROAD FOUND.** The
count is written this way deliberately, and this is the sentence earning it: the
row used to say "nine of the ten", predicting that "a tenth road found tomorrow
makes this row 'nine closed, a tenth found' instead of making it false". Both
open roads want a maintainer decision before anyone starts — the absolute
`SimTick` (`Q128`, netcode) and the snapshot schema fingerprint hashing English
prose (`Q122`, found 2026-09-16). Nine closed, two open.

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
| **the snapshot schema fingerprint** | ⛔ **OPEN, AND BLOCKED ON A MAINTAINER — `Q122`.** `schema_dump()` emits a prose `detail` per row and `compute_schema_fingerprint` hashes the whole dump, so English wording is inside the identity `ActiveRollbackAuthority::installed` gives a timeline. Measured by poison: pluralising ONE WORD in `detail::MESSAGE_CLEAR` turns the baseline red with 166 diff lines, 83 added and 83 removed. That is host-local lineage in a peer-stable identity in its purest form — two builds of the SAME mechanical schema are two identities if somebody reworded a comment. ⚠ The naive fix is refuted: of 493 rows, 268 carry facts `kind` does not encode (entity handle vs SET vs keyed MAP remapping, identical vs presence-aware canonical checksums, 22 custom-checksum descriptions), so dropping `detail` would stop the fingerprint seeing an entity-remapping change. The shape is a split, and where the line falls is the decision. ⇒ Landed meanwhile without needing it: the 15 sentences had TWO owners across two crates with nothing comparing them, and now have one (`879a5a1a3`, dump byte-identical) |
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

**The 14 are triaged — DONE 2026-09-16.** Six took waivers with citations; the
other eight are filed as `MENU-RESET-MIDSESSION` and `DURABLE-HORIZON-CHECKSUM`,
each with a named experiment. The guard stays RED on those eight, which is
correct: waiving them would move a count and answer nothing.

⛔ **ONE OF MY OWN READINGS BELOW WAS WRONG AND IS CORRECTED IN PLACE.** I
recorded all 7 menu systems as REAL because they carry
`.run_if(simulation_authorized)` — an argument about WHEN they run, which says
nothing about WHETHER they write. Five of the seven cannot reach the write at
all. ⇒ A run condition is not a reachability proof, and I reached for it because
it was the fact I already had.

| finding | reading |
| --- | --- |
| `reset_inventory_on_new_game` | **REAL.** Producer `process_new_game_reset_request` runs in `sim_schedule()` under `ResetProcessing`; consumer runs in `Update`; `NewGameResetCommitted` is `clear_message_on_rollback`. Being moved onto its sibling `clear_transient_on_sandbox_reset`'s chain. |
| 7 menu systems → `NewGameResetRequested` | **TWO REAL, FIVE WAIVED — corrected from "all 7 REAL".** The flag is set only by `SystemMenuParams::request_reset`, whose one caller is `dispatch_menu_action`, whose only two callers are `grid_menu_action_activated` and `kaleidoscope_menu_action_activated`. The other five take the same bundle and never reach the field: the bundle over-grants. The two real ones are `MENU-RESET-MIDSESSION`. |
| 4 `AmbitionGameSave` writers → `DURABLE-HORIZON-CHECKSUM` | **REAL, AND A DIFFERENT FAILURE FROM THE ONE THE GUARD'S PROSE DESCRIBES.** That resource has a CHECKSUM projection, so two peers can disagree at ONE FRAME without anything drifting — the guard says "drifts a little further each time", which invites a reader to dismiss a one-frame disagreement. ⚠ `ambition_persistence/src/rollback_registration.rs` predicted exactly these in 2026-08-29: *"the ~6 systems that pair a non-rewinding `Local` edge-detector with these very resources would have failed SILENTLY once rollback went live."* |
| `adopt_occurrence_checkpoint_from_save`, `complete_durable_restore` → `DURABLE-HORIZON-CHECKSUM` | **UNRESOLVED, and the obvious argument does NOT transfer.** Both are one-shot latch-gated on `SaveRestored`, which looks like the activation waiver's "the write precedes the timeline". ⛔ But both also require a LIVE PRIMARY PLAYER BODY (`bodies.is_empty()` / `ready_body.single().is_err()`), and a live body means the session world root is live — which is the exact condition `maintain_local_session` gates GGRS start on. So they may run WITH a live timeline. They owe their own argument. |
| `track_versus_roster` | **WAIVED.** One write, in the `(on_versus, mine) == (true, false)` arm alone; every other combination falls through `_ => {}`, so it cannot write once the route has published its own roster. ⚠ `VersusMatch` feeds the peer checksum, so the waiver rests entirely on the write preceding the timeline — at route entry the roster is `Proposed` with nobody seated, and `maintain_local_session` starts GGRS only once a live body exists. |

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

⛔⛤ **A FOURTH WRITER OF `AmbitionGameSave`, AND IT IS NOT A MIRROR — THIS IS
THE SHARPEST OF THE SET.** `dispatch_pending_dialog_requests`
(`ambition_dialog/src/bridge.rs:125`, registered into `Update` at :53) calls
`save.data_mut().increment_dialog_visit(&dialogue_id)` when a dialogue starts.

⇒ The five systems above DERIVE the save from simulation state, so running them
twice writes the same bytes and running them zero times loses only freshness. An
INCREMENT has neither property. A rollback restores `AmbitionGameSave` to its
pre-increment value and `Update` does not re-run, so the visit is lost; if the
dialogue start is instead replayed through the sim, it is counted twice. ⚠ And
`AmbitionGameSave` feeds the peer checksum, so the two peers need not even
disagree about the dialogue to disagree about the number. ⇒ Answer this one
FIRST: it is the case where "derived from sim state, so it converges" — the
argument that makes the other five plausible — is simply not available.

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

### GUARD-CORPUS / ORPHAN-ARMS — CLOSED 2026-09-16

**Owner:** repo tooling. Both closed; kept only as the receipt other rows lean
on. Investigation is in git (`993fdef58`, `e660c2fc4`, `f8b878a55`, `faa2d84d1`,
`9acbb9947`).

**WHAT STANDS NOW.** `scripts/lib/test_paths.py` is the ONE answer to *"is this
Rust file test-only?"* and all five former copies call it. Every consuming check
carries a `POPULATION_FLOOR`, because widening an exclusion makes a check see
LESS and a check that sees less reports CLEANER — the floors are what let the
consolidation be reviewed at all, and two of the five went red on the repoint.
`scripts/tests/test_test_paths.py` asserts the orphan list is EMPTY.

⛔⛤ **THE STANDING RULE, AND IT IS THE HALF THAT WAS NOT DEDUPLICATION.** Every
copy tested a NAME. **What removes a file from a build is the `#[cfg(test)]` on
its `mod` line, which lives in the PARENT** — or an inner `#![cfg(test)]` at the
top of the file. A name and a build can disagree in BOTH directions, and both
were found here:
- `pattern/tests.rs` — 36 arms no `mod` line declared, so `cargo test` reported
  them as neither passed nor failed. Three weeks, zero runs. Declared; 36 passed.
- `enemy_projectile/mod.rs` — `mod tests;` and `pub(crate) mod test_support;`
  with NO gate, compiled into every RELEASE build, while the module's own doc
  called the namespace "test-only".
⇒ A guard for this class must resolve every file to its DECLARATION. A name rule
cannot see it, and `scripts/tests/test_test_paths.py` is where that lives.

⚠ **AND `ORPHAN-ARMS` CARRIED REAL COVERAGE, checked before deciding.** The
pattern types had moved to `ambition_characters::brain::boss_pattern`, whose
module has ZERO test arms and none of the 36 arm names — so the file was the only
test of that behaviour, not a stale duplicate. ⭐ A test sits with what it CALLS,
not with what it NAMES: the types moved, the tick functions did not, and the
dependency runs from `boss_encounter` to `characters`, so the test stays here.

⚠ **TWO SESSIONS IMPLEMENTED GUARD-CORPUS SIMULTANEOUSLY AND NEITHER KNEW** —
down to the same four measurements, one as `lib/test_paths.py` and one as
`lib/rust_sources.py`. <!-- cite-ok: the duplicate is named because it was DELETED; a resolvable citation would mean it still existed -->
⇒ A row with an owner and a "next implementation" says
nothing about whether somebody is in it RIGHT NOW. Say so in the row or in a
message before starting a step that takes hours.

### TEST-LANES — keep required test lanes executable

**Owner:** test runner / app integration lane.

**Current state:** the lane RUNS. `cargo test -p ambition_app --test app_it` →
**677 passed / 0 failed / 25 ignored of 702**, 234.97 s at `582186bff` on the
ToothbrushAmbition box. Missing prerequisites are reported as incomplete rather
than pass. ⚠ A suite total is stamped to a TREE **and a MACHINE**: two agents
disagreed by 98 arms for an hour because one checkout's gitignored sprite-sheet
publish output was ~90 files short. Name the box beside the number.

⇒ **THE LONG-RUNNING `app_it` FLAKE IS CLOSED (2026-09-16).** It was never a
flake: `b9f2ece18` gave `drive_boss_animators` `.in_set(WorldPrep)` to buy a
capability gate, while that system also runs
`.after(project_boss_attack_state_from_move)`, which is `.in_set(CombatSet::Playback)`
— LATER in the sim schedule. One system ordered both before and after Playback,
and the fixed loop retried the broken schedule forever, allocating ~27 MB/s for
the first minute and ~234 MB/s after. One orphaned arm reached anon-rss
64,629,160 kB in 316 s and took a 62 GB box down. Bisected over
`770ac4bff..ee3d0852e`; fixed at `23f786757`; guarded from BOTH sides by
`the_boss_animator_takes_the_gate_and_not_a_phase`. See git for the timeline.

⛔⛤ **THE THREE STANDING PROHIBITIONS IT LEFT.**
1. **A set carries a POSITION as well as a gate.** Adding `.in_set(X)` to a
   system that already has cross-phase `.after`/`.before` edges can contradict
   them. This is the INVERSE of
   [[reference_moving_systems_out_of_a_plugin_drops_their_set_membership]] and
   bites just as hard. Check which set every existing edge target lives in.
2. **A session gate is not a capability check.** `run_if(simulation_authorized)`
   answers *"is this session authorized"*, which is TRUE in a host that has no
   boss catalog. A system needing a resource is guarded by that resource's
   existence: `.run_if(resource_exists::<BossCatalog>)`.
3. **`cargo check` cannot see a schedule cycle, and neither can an arm that never
   steps.** `b9f2ece18` shipped on `cargo check` alone, with an explicit "NO
   `app_it` run" justified by those two facts — which were exactly what made
   `app_it` the only instrument that could see it.

⇒ **THE COMPOSITION PROBES REALLY STEP NOW** (`582186bff`).
`step_the_fixed_schedule` pins `TimeUpdateStrategy::ManualDuration(1/60)` AND
asserts a `FixedUpdate` counter is non-zero — the pin alone is not enough,
because anything that stops the loop advancing returns the arms to certifying a
build and that failure is SILENCE. Before the pin: 13 MB, 0.47 s, ZERO fixed
steps.

⇒ **THE `BodyWallet` RED IS CLOSED** (`4ccfef59c`) and it was a CROSSING, not a
schedule choice: `NewGameResetCommitted` is produced in the sim schedule's
`ResetProcessing` and is `clear_message_on_rollback`, so a rewind could clear the
trigger before its `Update` consumer ran. A waiver was never available.

⭐⭐ **A ROLLBACK-MUTATOR RED HAS THREE INDEPENDENT QUESTIONS BEHIND IT, and
answering one is not a verdict.** (1) is the write inside the rewind window;
(2) is the write at a point no rewind CROSSES, which satisfies the guard without
moving anything; (3) is the TRIGGER erasable by a rollback, which closes the
WAIVER route and which (2) cannot rescue. ⚠ Applying (3) to a peer's road would
have told them they were clear of a charge they had not answered.

⛔ **OPERATIONAL RULES FOR THIS LANE, kept because they cost a night.**
- `scripts/measure_test_arm_rss.py` bounds a runaway: one process per arm (peak
  RSS is a property of a PROCESS), `RssAnon` rather than `VmRSS` or cgroup
  `memory.current`, kill by process group at a hard cap, and it refuses a row
  where libtest ran zero tests.
- ⛔⛔ **`pkill -f <pattern>` IS NOT A SAFE CLEANUP.** The shell running it is a
  `bash -c '<whole line>'`, so its own argv contains the pattern and the first
  `pkill` kills the shell — the second one, aimed at the binary, never runs, and
  neither does the verifying `pgrep`. That is how a 61.6 GB orphan escaped a
  sampler whose cap was working. ⇒ `pgrep -af` to LIST, kill by PID, re-`pgrep`
  in a SEPARATE call.
- ⚠ When a build fails in a crate you did not touch, check free space BEFORE
  reading the diagnostic. ENOSPC arrives as `error: could not compile <crate>`
  with the cause one line above, and has been seen as six ordinary-looking
  compile errors with no `os error 28` anywhere.

**Still open.** One non-reproducing session-root handoff failure whose assertion
message was never captured. On the next reproduction, capture the full failing
assertion and isolate the production ordering/state source before changing test
ordering or adding retries. Keep compile-cost and prerequisite failures distinct
from behavioural flakes, and from CONTENTION — a coherent measured story that
fits the first observation is still the wrong one if it was never tested against
a second.

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.
