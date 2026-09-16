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

⇒ **POST-A10 DEMOLITION IS DONE ON THE SYMBOL AXIS (2026-09-16).** MEASURED: every
piece of A10 machinery has live production callers — `PendingConstructionReceipt`,
`FrozenPublicationEffects`, `PublicationRetention`, `PendingWorldReplacement`,
`RoomCommitStamp`, `opening_refused`, `entity_is_still_a_candidate`,
`publications_holding_frozen_effects`. Nothing in that set is scaffolding left
standing. The dead mechanism the demolition DID find — `CandidateState` /
`spawn_candidate_state` / `candidate_state_entities`, zero callers, comments
specifying a road production never took — is deleted (`09629b060`).

⚠ **AND THE PUBLIC-SURFACE AXIS IS ESSENTIALLY CLOSED TOO.** Of every `pub`
item in `transaction.rs` and `stage.rs`, exactly ONE had no caller outside those
two files: `RoomConstructionPlan::predicted_authoritative_ids`, now private.
`RoomConstructionPlan` is a public type, so a public accessor on it is public API
whether or not anyone outside uses it.

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

### ROLLBACK-KIND-SPELLING — one registration, one kind, spelled once — ✅ DONE 2026-09-16

**Receipt:** `ambition_platformer2d_core::rollback_kind::spelling` holds all
**18** (kind, sentence) pairs; both roads reference the const and neither spells
a kind literal beside a sentence any more. Guarded by
`scripts/check_rollback_kind_spelled_once.py`, wired into `--maintenance`
(9 jobs) with `scripts/tests/test_rollback_kind_spelled_once.py` beside it so it
also runs under `pytest scripts/tests`.

⭐ **BYTE-EXACT: the collapse changed NOTHING.**
`the_rollback_schema_matches_its_recorded_baseline` passed unchanged, and
`compute_schema_fingerprint` hashes the whole `schema_dump()` including `detail`,
so that is proof rather than corroboration. POISONED: setting
`spelling::MESSAGE_CLEAR.kind` to `ComponentClone` REDDENS the baseline — and
raises NO conflicting-registration error, which is the acceptance itself. There
is no longer one road to change.

⭐⭐ **THE MEASUREMENT THAT MADE IT SMALL: KEY ON THE PAIR, NOT THE METHOD.**
Across both roads there are exactly 18 distinct literal (kind, detail) pairs and
each occurs EXACTLY TWICE — a perfect 1:1, zero disagreements. Two of my own
parsers got the METHOD attribution wrong (one invented four recording-only
methods; another swallowed the file tail into the last method and reported three
disagreements that did not exist). The pair needs no attribution at all, so the
edit is 36 mechanical substitutions rather than a trait redesign.

⛔⛤ **AND THE COSTED DESIGN THIS ROW CARRIED DOES NOT TYPECHECK. MEASURED
AGAINST `rustc`, NOT ARGUED.** The row proposed ONE required
`install<T>(owner, name, kind, detail, ops)` primitive with 24 default bodies.
The methods' `T` bounds are DISJOINT — `SnapshotState` vs `SnapshotCursor` vs
`SnapshotResolve` vs `MapEntities`, and `Component` vs `Resource` — so
`install`'s own bound list must be their UNION and every default body fails
`E0277` at the call. A 30-line probe compiled that shape and got exactly that.
The shapes that DO typecheck either reintroduce ~21 op types (the cost this row
already rejected) or require `core` to name the host's `App`, which is the
dependency the two-road split exists to prevent.

⇒ **A COSTED DESIGN IS STILL A REASONED ONE.** This row priced a shape carefully,
rejected the alternatives on size, and never compiled it. The 30-line probe that
refuted it was cheaper than the paragraph that proposed it.

⚠ **WHAT IS DELIBERATELY NOT COLLAPSED.** The `*_custom_checksum` family takes a
caller-supplied `detail`, so it names a kind with no literal sentence beside it
and has nothing to share. The guard does not flag those, and demanding a const
for each would be a table of one-element rows.

⚠ **DEMOTING THE GENUINELY-`derived` REGISTRATIONS IS NOT PART OF THIS AND WAS
NOT DONE.** Of 212 types registered through a `*_clone` method, 21 have a
declaration doc matching `derived|recomputed|never authored|never persisted`, and
reading them, most say "derived" about something ELSE — `ActorRenderSize`'s
COLLISION BOX, `CapturedBy`'s INVERSE. The ones that really do describe
themselves that way are deliberate, and the reason is written at
`crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs:428`.
⭐ THE RULE: a component whose PRESENCE is read by a query filter is
AUTHORITATIVE even when its value is derived. The demotion population looks close
to zero and nothing should be demoted on a keyword match.


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

**Current state (2026-09-16): THE CONSTRUCTION-IDENTITY HOLE — THIS ROW'S TITLE
AND WHOLE SCOPE — IS CLOSED.** ⚠ That is NOT all of A2: the work frontier's
[A2a/A2b/A2c](engine/actor-monolith-work-frontier.md) are the geometry, obstruction
and recipient-naming contracts, and they are a different subject with a different
normative owner. A reader who takes "A2 closed" from here and applies it there
will be wrong. The swept-contact resolver, finite
obstruction, exact ordering, targeted delivery and compound solid-contact policy
were already established, with build-site census coverage. The two identity roads
this row existed for are closed and the acceptance is met — the player clone
(`ADR 0030`, one site) and the five dynamic-mint fallbacks (`_ => None` at every
bare `match` over `SimId::spawned`), with `UnmintedBodyCensus` naming the
construction ROAD in its witness.

⚠ **TWO MINT SITES DEGRADE ON PURPOSE AND STAY,** and that is a count, not a
completeness word: `ambition_held_items`'s thrown-item mint and
`puppy_slug_gun`'s minion mint are `.ok().map(..)`, marked at the site as the
visible edge of the unclosed inventory leg on `ItemCustody`. They belong to that
leg, not to this row. ⇒ A sixth bare-`match` site found tomorrow makes this
"five closed, a sixth found" rather than making the row false.

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

⛔⛤ **THE ROW SAID THREE SPAWN OWNERS. A CENSUS OF EVERY `SimId::spawned` CALL
SITE FOUND FIVE.** `crates/ambition_boss_encounter/src/encounter_script.rs` (the
`DropHazard` beat) and `game/ambition_content/src/portal/fire_adapter.rs` (the
portal shot) carried the identical `_ => None` fallback and were not in the row's
list. All five are closed now. ⇒ The row's "three" was a FLOOR that did not say
so; the population is `grep -rn 'SimId::spawned'` over `crates/` and `game/`,
minus the tests, and it is 5 bare-`match` sites.

⚠ **AND TWO MORE MINT SITES DEGRADE DELIBERATELY, WITH THE REASON WRITTEN AT
THEM.** `ambition_held_items`'s thrown-item mint and `puppy_slug_gun`'s minion
mint both `.ok().map(..)`, and the first says why: *"This arm is the visible edge
of the unclosed inventory leg described on `ItemCustody` — not a fallback that
should quietly absorb the common case."* Those are decided placements, not the
pattern this row closed; converting them would close a caller's hole by deleting
another row's marker.

⇒ **AND THE PORTAL REFUSAL HAD TO CONSUME THE PRESS.** `melee_pressed` is cleared
at the bottom of that loop so the wearer's jab does not answer the same press. A
refusal that skipped the arm would leave the press set and the body would JAB
instead of nothing happening — so the refusal clears it before `continue`. Same
class as the mana ordering below: a refusal inherits every side effect the arm it
skips was responsible for.

✅ **THE UNNAMED SPAWNER IS CLOSED, 2026-09-16.** `materialize_matching`
(`ambition_projectiles/src/materialize.rs`) inserts no identity and defers to
`mint_spawned_sim_ids`, which is the designated late mint and was never the hole.
The hole was its input: `fire_sentry_system`, the vortex cast and
`tick_gravity_grenade_fuses` each computed the spawned id through a `match` whose
fallback arm was `_ => None`, so an unnamed turret deployed and
`mint_spawned_sim_ids` then skipped every bolt it fired — a bolt mints under the
turret. All three are `let (Some(..), Some(..)) = .. else { warn!(..); .. }` now,
the same shape the clone road uses.

⛔⛤ **THE ORDER WAS HALF THE FIX AND IT IS NOT OBVIOUS FROM THE ROW ABOVE.** In
the sentry and vortex systems `mana.meter.try_spend(..)` runs in the same loop
body, ABOVE where the id was computed. A refusal written where the `match` was
takes the caster's mana and spawns nothing — strictly worse than the defect. Both
refusals sit above `try_spend` now, and the guard asserts the METER as well as the
turret count:
`a_deployer_with_no_identity_deploys_no_turret_and_keeps_its_mana`
(`ambition_abilities::ranged::sentry::tests`). ⚠ Poisoned by moving `try_spend`
back above the refusal — the mana assertion fails with the message naming that
exact edit. Its control is `the_same_deployer_with_its_identity_does_deploy`,
because "no turret" is otherwise satisfied by a dozen unrelated gates in that
loop.

⚠ **THE GRENADE REFUSES BUT STILL DESPAWNS.** Its fuse has already expired when
the id is needed; skipping the whole arm would leave a spent grenade retrying
every tick forever. The refusal costs the EFFECT, not the cleanup.

⭐ **THE SEAM SIGNATURES KEEP `Option<SimId>` DELIBERATELY, and that is a decision
already recorded at `open_vortex_well`:** *"`id` IS `Option` AND THAT IS NOT A
HEDGE. A well minted under a caster the sim can name gets `SimId::spawned`; a
fixture well has no caster to mint under."* The row's target was the production
CALLERS' fallback, not the seams — tightening the seams would have deleted a
documented fixture road to close a caller's hole.

⇒ **AND THREE FIXTURES WERE EXERCISING THE ROAD THAT NO LONGER EXISTS.** Four
tests reddened, all because `spawn_primary_player_holding` built a body with no
`SimId` and no `SimIdCounter` while `ensure_sim_id` gives every production body
both before `CoreSimulation`. The fixture carries them now, so those tests take
the production path; the grenade fixture likewise. That is the fix, not a
workaround: a fixture that can only reach the degraded road cannot witness the
real one.

✅ **ACCEPTANCE MET 2026-09-16 — THE WITNESS NAMES THE ROAD.** A MECHANICAL body
(`BodyKinematics`, not merely a damageable one) cannot reach the simulation
unnameable, and `UnmintedBodyCensus` now says WHICH and BY WHAT: each entry in
`skipped_bodies` carries the entity, its `Name` and its `SpawnOrigin`. The origin
is the load-bearing field — an id and a name say which body, only the road says
where the repair goes, and the sweeper's own comment says the repair belongs at
the spawn site.

⛔ **THE FIELD WAS INVISIBLE TO EVERY PASSING RUN, WHICH IS WHY IT HAS ITS OWN
CONTROLS.** A healthy tree reports `0 skipped over 0 distinct bodies`, so the
formatting of a populated entry is never exercised by the two live consumers.
Four arms in `ambition_platformer2d_runtime::sim_identity` cover it directly:
`a_skipped_body_is_named_with_the_road_that_built_it` (a `ProviderStaged` body,
asserting both the name and the provider/instance appear),
`a_body_whose_road_recorded_nothing_says_so` (⚠ an absent `SpawnOrigin` is a
FINDING, not a blank — a road that recorded nothing is a different repair from one
that recorded the wrong thing), `an_identified_body_is_not_named` (the control,
without which every arm above is satisfied by a census that records everything),
and `the_set_caps_and_admits_it`.

⚠ **THE SET IS CAPPED AT 16 AND `capped` SAYS SO**, because an uncapped set in a
600-frame run is a memory leak in an instrument and a capped one that does not
admit it is a total that quietly stopped counting. Past the cap its length is a
FLOOR; `skipped` keeps counting observations and is the field that is not.

⇒ Poisoned both ways: redacting the origin from the descriptor reddens exactly the
road arm; removing the cap reddens exactly the cap arm. Reverted from a `cp`
snapshot and byte-compared. ⓘ The two `app_it` consumers print the whole set
rather than a `first`, and both pass (2 and 6 arms).

✔ **THE CLONE-ROAD RECEIPT IS RE-MEASURED ON TODAY'S TREE AND THE HANG IS GONE.**
It was measured at the pre-merge tree `b9f2ece18`, and at `ecbdf2297` no `app_it`
test that steps the simulation terminated — 3.53s before the merge, not finishing
in 300s after it. Re-run 2026-09-16 at `dae0fc44a`:
`the_player_clone_road_builds_an_identified_body` passes in **1.60s** and prints
`209 body-observations judged, 0 skipped`, the same numbers the original receipt
claimed. The whole `-p ambition_app` suite finishes: 213 + 678 + 1 passed, 25
ignored, 375s. ⇒ The receipt is now a claim about `main`, and the deferral above
it is discharged rather than restated.

### A12 — finish move-contact attribution and reflection identity

**Owner:** [`engine/authored-technique-admission.md`](engine/authored-technique-admission.md)
and combat/projectile occurrence identity.

**Current state:** ranged feedback carries `MoveOccurrence` end to end, so a
projectile launched by move A cannot be credited to whatever move happens to be
playing when it lands. Melee stamps use the same occurrence authority.

✅ **THE GUARD THIS ROW ASKED FOR ALREADY EXISTS — re-read 2026-09-16, and the row
was the stale half.** *"A body which has started a move cannot lose
`MoveOccurrence` during ordinary body lifetime"* is
`every_playing_body_kept_its_occurrence` in `ambition_combat::moveset::tests`,
called from `a_body_that_has_started_a_move_never_loses_its_occurrence` across a
move, an idle gap and the next move. It is the one-directional form — **a body
carrying `MovePlayback` carries `MoveOccurrence`** — with an anti-vacuity floor
counting PLAYBACKS (the population the invariant is about), and two poison arms:
`removing_the_occurrence_mid_move_is_caught` breaks the PROPERTY rather than the
assertion, and `the_guard_refuses_a_world_with_no_body_mid_move` poisons the
floor. 4 arms, green.

⭐ **AND THE ROLLBACK HALF OF THE ACCEPTANCE IS A MEASURED CHAIN, NOT AN
ASSUMPTION.** `MoveOccurrence`'s doc claims *"a rewind that kept a later count
would make the resimulated move claim a number the abandoned future spent… it is
rollback-registered"*. Both links verified:

1. It is `component-canonical` (`rollback_schema_baseline.txt:48`), so a rewind
   restores the exact value AND two peers compare it.
2. Its only writer is `start_move`, reached from `trigger_moveset_moves`, which is
   registered through `app.add_systems(sim, ..)` in
   `runtime/src/combat_schedule.rs` — the REWINDING schedule. Confirmed positively
   at the registration, not inferred from the mutator guard's silence, and
   `check_rollback_mutators_run_in_sim.py` independently does not list it among
   its 8 offenders.

⇒ That is the defect class `a_bag_changed_from_update_is_silently_taken_back_by_the_rewind`
found for `OwnedItems`: a player-visible write from outside the rewinding
schedule, restored away with nothing reporting it. `MoveOccurrence` is not exposed
to it, because its writer is inside.

✅ **AND THE VALUE-LEVEL ROLLBACK WITNESS LANDED 2026-09-16.**
`a_move_occurrence_reaches_the_same_number_with_and_without_a_rewind`
(`game/ambition_app/tests/a_move_keeps_its_occurrence_across_a_rewind.rs`) drives
the SAME world for 180 frames with a GGRS sync-test session and without one, and
compares the number the primary player's counter reaches. **Good value: `Some(9)`
and `Some(9)`** — nine moves at one press every twelfth frame.

⛔ **POISONED WITH THE PROPERTY, NOT THE ASSERTION.** Removing
`rollback_component_canonical::<MoveOccurrence>` makes it fail `Some(1)` vs
`Some(9)`: unregistered, every rewind drops the counter, so the body never gets
past its first move while the fixed-tick host reaches nine. ⚠ Until this arm
existed, nothing in the repository failed when that registration went away except
the schema baseline — which would only have said *the dump changed*, not *the
identity stopped surviving a rewind*.

⇒ That is why it is a third KIND of evidence rather than a third measurement. The
two structural links each had a guard; neither guard reads the NUMBER, and
`ambition_combat`'s own four arms run on a hand-built App with no rollback session
at all — the exact shape that hid the `OwnedItems` defect.

**Remaining engineering:** finish reflection/contact attribution after the product
rule is settled — blocked on `Q101`, below.

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

⇒ **AND HALF OF THIS ROW'S ACCEPTANCE WAS SILENTLY UNMET UNTIL 2026-09-16.** It
asks that each supported profile *"constructs and STEPS a real subject"*. The
three composition probes in `composes_through_the_sdk` called `app.update()`
eight times under `TimeUpdateStrategy::Automatic`, which `add_headless_foundation`
leaves in force through `MinimalPlugins` — so `FixedUpdate` ran zero or more
times depending on WALL TIME, and a fast run crossed 1/60 s never. MEASURED
before the fix: 13 MB peak, 0.47 s, **ZERO fixed steps**. They certified that the
engine BUILDS.

`582186bff` pins the step AND asserts a `FixedUpdate` counter is non-zero, so the
"steps" clause is now real for the three probed compositions (cutscenes, portals,
boss encounters). ⚠ The pin alone was not enough: anything that stops the fixed
loop advancing returns these arms to certifying a build, and that failure is
SILENCE rather than a red.

⭐ **THE GENERAL FORM, worth more to this row than the fix:** an arm whose green
is compatible with the engine being broken certifies nothing, and "it passes
quickly" is the tell. A profile contract needs a witness that STEPS, and a
witness that steps needs a witness that it stepped.

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

### D-SCENARIO-IDENTITY — CLOSED 2026-09-16

**Owner:** performance/scenario tooling. Closed by measurement; nothing was
implemented. Detail in git.

⚠ **THE ROW SAID THE SUBJECT WAS NOT IN THE TREE. IT WAS — IN A SUBMODULE.**
`CombatScenario.cache_name()` and `scenario_key()` live in
`tools/ambition_moveset_inspector/`, which a source inspection confined to
`crates/` and `game/` cannot see. ⇒ Check `tools/` before believing the next
"not located in the tree".

The acceptance is met by a stronger mechanism than the row proposed: the cache
refuses any entry whose repository content differs at all, because
`_evidence_is_current` requires `source_identity == _repository_identity()` —
`HEAD` plus `sha256(git diff HEAD + git status --porcelain -uall)`. MEASURED by
moving content and reading it back: editing a crate source moves it, editing the
SUBMODULE moves it too (through the dirty-submodule line), and restoring returns
the original digest exactly.

### ROLLBACK-DEAD-SESSION — an invalidated GGRS session stops the clock in silence

⛔ **A SYNC-TEST SESSION THAT INVALIDATES KEEPS ACCEPTING `sim.step()` AND STOPS
ADVANCING `SimTick`.** The step returns an observation every time. Nothing
panics, nothing prints, and every assertion after the invalidation runs over a
frozen world — where it agrees with itself, forever.

MEASURED 2026-09-16 (`probe_how_far_each_harness_ticks_over_the_same_window` in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`), `SimTick`
over 240 `sim.step()` calls:

| harness | tick at 0 / 40 / … / 240 | `session_health` |
|---|---|---|
| `new_with_options` sync-test | 1, 41, 81, 121, 161, 201, 241 | `Ok` |
| no rollback session | 0, 40, 80, …, 240 | `Ok` |
| `build` + compose, EMPTY system | 1, 41, 81, 121, 161, 201, 241 | `Ok` |
| `build` + compose, a bare `OwnedItems` write | 1, 6, 6, 6, 6, 6, 6 | `Err(checksum mismatch at frames [2, 3, 4, …])` |

⇒ The freeze is not the compose road and not the schedule: an empty system
through the same callback ticks 1:1. It is one system writing a
rollback-registered resource (`OwnedItems`, `rollback_resource_clone`,
`crates/ambition_items/src/rollback_registration.rs:11`) outside its sanctioned
road, which desyncs the sync test — and the desync then presents as a stopped
clock rather than as a failure.

**THE EXPOSED POPULATION IS ZERO, AND THE FIRST COUNT SAID SIX.** Of the 21
files built on `with_sync_test_rollback_settings`, thirteen call
`rollback_health()` or `session_health`. I filed the other eight as exposed. Then
I read them, and every one of them already refuses a frozen world — not with a
health check, but with an assertion a stopped clock cannot satisfy:

| arm | what a dead session breaks |
|---|---|
| `canonical_state_is_finite.rs` | population floor: `finite_seen >= ENCODED_FLOAT_FLOOR` against a measured 116,280 |
| `input_stream_under_rollback.rs` | recorded stream length compared against the tick count |
| `rollback_provoked_actor.rs` | `load_runs` must move; `assert_rolled_back` |
| `d71_transaction_census.rs` | explicit preconditions `room_changes > 0` and `transactions > 0` |
| `carried_item_crosses_rooms.rs` | `walk_through_the_door_to` panics after 60 frames with no room change |
| `door_entry.rs` | asserts the room changed after the authored hold |

⭐ **THE TRANSFERABLE PART IS THAT `grep` FOR THE HEALTH CALL MEASURED THE WRONG
THING.** "Does this arm ask whether the session is alive" and "can this arm pass
over a dead session" are different questions, and only the second one matters. An
arm that demands a room change has a better liveness check than one that reads
`rollback_health()` once at the end, because its check is load-bearing for what
the arm is actually about. ⇒ Counting calls to a safety API measures vigilance;
counting assertions that a broken world fails measures safety.

⚠ So there is no cleanup here and NOTHING SHOULD BE EDITED IN THOSE SIX FILES.
Adding `rollback_health()` to them would add a redundant check and would trade a
strong guarantee for a visible one.

⇒ WHAT REMAINS IS THE CONTRACT, NOT A CLEANUP. The tree is currently safe by
accumulated good taste in individual arms, and nothing holds that property in
place: the next rollback arm written is exposed the moment its assertions happen
to be satisfiable by a frozen world, and its author gets no warning.

0. ⚠ **THE CENSUS ABOVE ROTS.** It proves the CURRENT 21 arms are safe; it says
   nothing about the twenty-second. The cost of not fixing the contract is that
   every future rollback arm inherits the exposure and its author gets no
   warning — which is the same argument that turns "the only production
   registrar is this one" into an absence contract rather than a note.
   (ToothbrushAmbition's point, and it is the reason this row stays open after
   the exposure count went to zero.)
1. Decide whether `Platformer2dSimHarness::step` should refuse to step an
   invalidated session rather than leaving every caller to notice on their own.
   ⭐ That is the real fix: the current contract makes silence the default. It
   touches `crates/ambition_sim_harness/src/runtime.rs::step`, so it wants a
   maintainer ruling — a harness that panics on a dead session will red any arm
   that turns out to be relying on one, and the census above says none is.
2. ⚠ A guard script is the WRONG shape here and the table above is why. The
   property is "this arm's assertions are unsatisfiable by a frozen world", which
   is not decidable by reading the source — six different mechanisms produced it
   and a seventh would too. ⇒ If the contract moves into `step`, no guard is
   needed; if it does not, no guard can be written.

### ROLLBACK-BAG-DESYNC — a per-tick change to an UNHASHED resource desyncs the sync test

⛔ **CHANGING `OwnedItems` ONCE PER TICK DESYNCS A GGRS SYNC TEST WITHIN SIX
TICKS, BY EITHER ROAD.** The mismatch repeats at frames `[2, 3, 4]` forever and
presents as a frozen clock (see
[ROLLBACK-DEAD-SESSION](#rollback-dead-session--an-invalidated-ggrs-session-stops-the-clock-in-silence)).

MEASURED 2026-09-16, `probe_how_far_each_harness_ticks_over_the_same_window` in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`, `SimTick`
over 240 `sim.step()` calls:

| system added to the sim schedule | tick at 0 / 40 / … / 240 | `session_health` |
|---|---|---|
| none (`new_with_options`) | 1, 41, …, 241 | `Ok` |
| none, no rollback session | 0, 40, …, 240 | `Ok` |
| empty system through `compose` | 1, 41, …, 241 | `Ok` |
| `ResMut<OwnedItems>`, **grants ZERO** | 1, 41, …, 241 | `Ok` |
| `ResMut<OwnedItems>`, grants 1/tick | 1, 6, 6, 6, 6, 6, 6 | `Err(mismatch [2, 3, 4, …])` |
| `MessageWriter<ItemGrantRequested>`, 1/tick | 1, 6, 6, 6, 6, 6, 6 | `Err(mismatch [2, 3, 4, …])` |

⇒ **THE TRIGGER IS THE VALUE MOVING, NOT THE WRITER.** The zero-grant row is the
control that settles it: same `ResMut<OwnedItems>`, same unordered position in the
schedule, same change detection, bag value unchanged — and it ticks 1:1. And the
sanctioned road desyncs identically to the direct write, so this is not a case of
writing rollback state from the wrong place. `ItemGrantRequested` →
`apply_item_grants` is the engine's own road and the message is
`clear_message_on_rollback`.

⛔ **AND `OwnedItems` IS NOT HASHED**, which is what makes this sharp rather than
routine. It is registered `rollback_resource_clone`
(`crates/ambition_items/src/rollback_registration.rs:11`), and
`RollbackEntryKind::feeds_peer_checksum` returns FALSE for `ResourceClone`
(`crates/ambition_platformer2d_core/src/rollback_kind.rs:87`): the bag is
snapshotted and restored, never compared. It cannot itself be the value the two
passes disagree about. Something hashed is deriving from it.

⭐⭐ **ANSWERED 2026-09-16: IT IS `AmbitionGameSave`, AND THIS IS EXACTLY THE
MECHANISM [DURABLE-HORIZON-CHECKSUM](#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update)
PREDICTED.** Asked of the registry rather than guessed a fifth time —
`RollbackChecksumProbes::census_all` reads every entry that feeds the peer
checksum, so running both harnesses to the same tick inside the live window and
diffing their censuses names the value that follows the bag:

```
after 5 steps: granting bag=10 tick=6, still bag=3 tick=6
≠ ambition_persistence::save::AmbitionGameSave: granting=(1, 0xd41e15e06646859b) still=(1, 0x8f605a278dac557d)
1 of 364 probed entries differ
```

**ONE ENTRY OF 364.** The chain is now end to end:

1. `persist_inventory_to_save` runs in **`Update`** — once per FRAME
   (`crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs:296`).
2. It writes the live bag into the save: `save.data_mut().set_inventory(items, wallet.balance)`
   (`crates/ambition_platformer2d_actor_monolith/src/items/persist.rs:161`). Measured
   tracking the bag 1:1 — `healthcell` count 5, 6, 7, 8, 9, 10 as the live bag goes 5 → 10.
3. `AmbitionGameSave` is `rollback_resource_clone_checksum`
   (`crates/ambition_persistence/src/rollback_registration.rs:31`), and its checksum
   serializes the WHOLE save to RON (`save.rs:52`), so ANY field moves it.
4. A rewind re-simulates ticks and does NOT re-run `Update`. The hashed value
   therefore describes a different frame from the tick it is compared at.

⇒ Which is why it needs the bag to CHANGE. A value put back before it was taken
cannot be disagreed about — the zero-grant control is that case, and it is clean.

⚠ **THREE ELIMINATIONS HOLD; THE FOURTH WAS A FALSE NEGATIVE AND IT WAS MINE.**
I reported the save mirror eliminated because `mirrored_items()` filtered the
save's item list for the substring `"HealthCell"` and counted 0 across the whole
window. The save stores `PersistedItem { id: "healthcell", count: N }` —
lowercase, an authored id rather than the enum's `Debug`. The resource was
changing the entire time and my observable could not see it. ⇒ **A projection
that reads nothing and a resource that holds nothing are the same reading**, and
that is the same defect as a guard matching no files and a session frozen at tick
6: the instrument agreeing with itself. The census could not make this mistake
because it asks the registry for every entry by type rather than asking one
resource for a shape I guessed.

**STILL ELIMINATED, each by measurement:**

1. **NOT a drained-message edge.** `capture_owned_items_baseline` writes the
   checksummed `OwnedItemsBaseline` from the sim schedule gated on
   `MessageReader<CheckpointCommitted>` — the "drained in the original pass,
   empty in the replay" shape — but that channel IS `clear_message_on_rollback`
   (`crates/ambition_platformer2d_shared_tangle/src/lifecycle/horizon.rs:172`).
   The census agrees: `OwnedItemsBaseline` is not among the entries that differ.
2. **NOT the system's presence.** The zero-grant control.
3. **NOT change detection.** `owned.grant(item, 0)` takes `ResMut` and calls a
   `&mut self` method, so it derefs mutably and marks `OwnedItems` changed every
   tick exactly as the granting version does. A `Changed<OwnedItems>` filter would
   fire identically in both, and only the row whose VALUE moves desyncs.

⚠ **THE LIVE WINDOW IS FRAMES 0–5, NOT 240.** The mismatch is reported at
`[2, 3, 4]` and the freeze is downstream of it, so anything measured after the
invalidation is a frozen world agreeing with itself. `session_health` is clean at
steps 0 through 5 and first reports at step 6.

⇒ **AND THE SAME ANSWER ARRIVED BY A SECOND METHOD, WHICH ADDS ONE FACT THE
CENSUS DIFF CANNOT SHOW: `Update` IS THE WRITER, MEASURED AT THE REWIND
BOUNDARY.** `RollbackRestoreAudit` compares frame F's census against frame F's
EARLIER census within one run, so it reads the resimulation itself:

```
frame 2: AmbitionGameSave  first xor 0x4f52c70a…  on replay 0xce4e4758…
frame 3: AmbitionGameSave  first xor 0x8cf64e57…  on replay 0xce4e4758…
frame 4: AmbitionGameSave  first xor 0xe2f498aa…  on replay 0xce4e4758…
```

⭐ **THE REPLAY XOR IS CONSTANT WHILE THE FIRST-PASS XOR MOVES EVERY FRAME.** That
is step 4 of the chain above turned into a reading: a resimulation re-runs the sim
schedule and not `Update`, so every replay of every frame sees whatever the LAST
frame's `Update` wrote — one value, repeated. A two-run diff at one tick shows
that the entry follows the bag; only the rewind boundary shows that the replay
stops updating it.

⇒ And the row-count half of the false negative, for the record beside the
substring half: `PersistedItem` is `{ id, count }`, so granting the same item
moves a `count` and leaves the ROW COUNT alone — `rows` is **7 in both arms at
every step** while the summed quantity climbs 13 → 17 under granting and holds at
10 in the control. Either reading alone would have said "not changing".

**Pinned, so neither answer needs re-deriving:**
`exactly_one_hashed_entry_diverges_when_the_bag_moves_and_it_is_the_save`
(`game/ambition_app/tests/which_hashed_entry_moves_when_the_bag_does.rs`) asserts
the diverging set is exactly `{AmbitionGameSave}`, floors the control audit's
`resimulations > 0` before reading its silence, and says in its own doc which
failure direction is the good one: no divergence means the repair landed; a second
type is a new finding; a diverging CONTROL means the cause is no longer the bag and
every elimination needs redoing.

ⓘ **TWO SESSIONS ANSWERED THIS INDEPENDENTLY AND IN PARALLEL, BY DIFFERENT
METHODS, AND AGREED.** Both probes are kept because they measure different things:

| probe | method | finds |
|---|---|---|
| `probe_which_hashed_entries_follow_the_bag` (`a_bag_changed_mid_window_reaches_the_save.rs`) | `census_all` of the granting run vs the still run AT THE SAME TICK | every hashed entry that DERIVES from the bag |
| `probe_which_registered_type_diverges_when_the_bag_moves` (`which_hashed_entry_moves_when_the_bag_does.rs`) | `RollbackRestoreAudit`, one run, frame F's census compared against frame F's earlier census | the entries that DISAGREE WITH THEMSELVES across a resimulation |

⚠ **ONE CORRECTION TO THE FIRST PROBE'S DOC, because it steered the choice of
method:** it says a difference across a rewind *"cannot be observed from outside"*.
It can, and the machinery for it shipped before either probe:
`record_saved_census` censuses every save and compares when GGRS saves the same
frame twice, which is what a resimulation is. That is where the constant replay
xor above comes from, and the constant is the evidence that identifies `Update` as
the writer — a two-run difference at one tick cannot see it.

⇒ NEXT. The mechanism is settled, so what is left is a product/architecture
choice and it belongs to DURABLE-HORIZON-CHECKSUM: a value derived once per frame
must not be compared once per tick. The two shapes are (a) derive the save inside
the sim schedule so a rewind re-derives it, or (b) take `AmbitionGameSave` out of
the peer checksum, since a save file is not simulation authority. ⭐ (b) is the
smaller change and probably the right one — but it is a claim about what peers
must agree on, so it wants a maintainer ruling rather than a patch.

The reproduction is committed and costs one `cargo test` to re-run:
`cargo test -p ambition_app --test app_it probe_which_hashed_entries -- --include-ignored --nocapture`
names the entry, and `probe_how_far_each_harness_ticks_over_the_same_window`
prints the full matrix.

⚠ **THE PROBES COST THE LANE NOTHING BECAUSE THEY DO NOT RUN IN IT.** They are
`#[ignore]`d and print-only; the assertions that hold this finding in place are
Yardrat's `exactly_one_hashed_entry_diverges_when_the_bag_moves_and_it_is_the_save`
and the three live arms in the same file. Full lane measured 2026-09-16 on the
no-GPU box: `cargo test -p ambition_app --test app_it` → 683 passed, 0 failed,
30 ignored, 726.66s.

⚠ **SCOPE, because it decides whether this is urgent.** A sync test is one
machine rewinding itself. If a single App disagrees with its own replay, no peer
is needed for the divergence, and every road that changes a bag during play —
pickups, shops, drops — crosses it. ⇒ But this is measured only for
`OwnedItems`; whether other `ResourceClone` entries behave the same way is
unmeasured, and the same probe answers it for any of them by swapping the system.

### DURABLE-HORIZON-CHECKSUM — the save mirrors write hashed state from `Update`

⭐⭐ **THE CENTRAL PREDICTION IS NOW MEASURED, 2026-09-16. It is no longer "can
the value hashed at a confirmed frame differ" — it does, it is
`AmbitionGameSave`, and it desyncs a sync test within six ticks.** A bag that
changes once per tick makes `persist_inventory_to_save` write a per-FRAME value
into a per-TICK checksum; a rewind re-simulates ticks without re-running
`Update`, so the hashed save describes a different frame from the tick it is
compared at. Of 364 probed rollback entries, exactly ONE differs between a run
whose bag moves and an otherwise identical run whose bag does not. The
measurement, the eliminations and the reproduction are in
[ROLLBACK-BAG-DESYNC](#rollback-bag-desync--a-per-tick-change-to-an-unhashed-resource-desyncs-the-sync-test).

⇒ **WHAT REMAINS IS A RULING, NOT AN INVESTIGATION**, and it is filed as
[Q129](awaiting-maintainer-decision.md#q129--must-the-save-file-be-part-of-what-two-peers-agree-on).
Either derive the save inside the sim schedule so a rewind re-derives it, or take
`AmbitionGameSave` out of the peer checksum on the ground that a save file is not
simulation authority. The second is smaller and probably right, and it is a claim
about what peers must agree on — so it belongs to the maintainer rather than to a
patch. ⚠ It settles three systems, not one: the other two `persist_*` mirrors
write the same resource from the same `Update` chain.

⭐ **AND IT TAKES A SUSTAINED CHANGE, NOT A SINGLE ONE.** A `SimTick`-gated grant
that fires once at tick 20 runs the full 240 steps clean — tick 241, health `Ok`
— with the bag column proving the grant fired (3 → 4 at step 40) rather than the
row passing for the wrong reason. ⚠ The gate is `SimTick` and not a `Local`
precisely because a `Local` is not restored by a rewind: the replay would skip a
grant the original pass performed and manufacture its own divergence. ⇒ So one
pickup does not desync on this evidence and the reproduction demonstrates the
sustained case. WHY they differ is unexplained and nobody should read "single
changes are safe" out of one measurement at one tick.

⚠ The dialog increment below is still costed and still unmeasured; nothing here
touches it.

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
FIRST on the argument, but NOT first with a test: it is the case where "derived
from sim state, so it converges" — the argument that makes the other five
plausible — is simply not available.

⚠ **THE TEST FOR IT IS THE EXPENSIVE ONE, MEASURED BEFORE ATTEMPTING IT.**
`dispatch_pending_dialog_requests` early-returns unless a `DialogueRunnerEntity`
exists, and `spawn_dialogue_runner` is itself
`.run_if(resource_exists::<YarnProject>)` — so reaching the increment needs a
compiled Yarn project in the harness, not just a stepped world. ⇒ The bag arm in
`a_bag_changed_mid_window_reaches_the_save.rs` is the cheap member of this class
and was done first for that reason; it is also the template, since the shape is
identical: change the value from outside the rewinding schedule mid-window, and
keep a no-rollback control beside it.

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

⭐ **AND THIS CLASS WAS PREDICTED IN WRITING, IN THE REGISTRATION THAT MAKES IT
CHECKABLE.** `crates/ambition_persistence/src/rollback_registration.rs` explains
why `AmbitionGameSave` was given a real content projection
(`AmbitionGameSave::checksum`) rather than a presence-only probe: *"the ~6
systems that pair a non-rewinding `Local` edge-detector with these very
resources would have failed SILENTLY once rollback went live."* ⇒ So the
instrument for this row already exists and is pointed at the right resource —
what was missing is an arm that makes the value CHANGE while the window runs.

⭐⭐ **THE FIVE ARE TWO PHASES AROUND A ONE-SHOT LATCH, AND ONLY ONE PHASE IS
THE PER-FRAME PROBLEM.** MEASURED 2026-09-16 by reading each guard clause:

| system | its own guard | so it runs |
| --- | --- | --- |
| `adopt_occurrence_checkpoint_from_save` | `if restored.0 \|\| bodies.is_empty() { return }` | ONCE, before the latch, and only with a live body |
| `complete_durable_restore` | `if restored.0 \|\| ready_body.single().is_err() { return }` then `restored.0 = true` | ONCE, as soon as a PRIMARY PLAYER BODY exists — it IS the latch, and it asks for a body, NOT for a save file |
| the three `persist_*_to_save` | `if !restored.0 { return }` | every frame AFTER the latch, value-compared |

⛔✦ **A CLAIM I PUT IN THIS ROW AND WITHDREW WITHIN THE HOUR, kept because the
wrong version is the one a reader would reach for.** I wrote that the mirrors
write nothing until a save has been RESTORED, so a harness booted with no save
file never flips the latch and any such test measures nothing. **Wrong.**
`complete_durable_restore` asks `ready_body.single().is_err()` and nothing else:
the latch flips as soon as a primary player body carries a `BodyWallet`, save
file or not. `AmbitionGameSave` is a plain `Res`, not an `Option<Res>`, so the
resource is always there to mirror INTO.

⇒ I inferred "needs a save" from the system's NAME and from the `save` field in
its signature, and never read its guard clause. The three `persist_*` really are
gated on the latch — that half held — but the latch is about a body.

⭐ **WHICH MOVES THE EXPERIMENT, AND MAKES IT SHARPER.** The mirrors run in
every harness that has a player, so `rollback_full_reset.rs` and
`rollback_lifecycle_reset.rs` already drive them for 180 and 240 frames and are
GREEN. That is not evidence they are safe: the mirror is value-compared, so in a
world where the bag never changes it writes once and then returns early forever.
⇒ The experiment is therefore NOT "boot with a save". It is **change the
mirrored value in the middle of the rollback window**, which nothing in the tree
does today, and assert the mirror actually wrote — before, during AND after the
window, since a value that is right at frame 0 and right at frame N may have
been lost and re-established in between.

⚠ **AND THE ONE-SHOT PAIR IS A NARROWER QUESTION THAN THE MIRRORS.** Both fire
in the window between a live body existing and the latch flipping — and a live
body is the exact condition `maintain_local_session` starts GGRS on, so the two
events are gated on the same fact and their order is not stated anywhere. That
is a RACE to characterise, not a per-frame accumulation.

⚠ **ATTEMPTED 2026-09-16 AND INCONCLUSIVE — THE HARNESS STOPPED SIMULATING.**
`probe_a_bag_changed_inside_the_sim_is_mirrored_across_the_window` in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs` adds a
grant system to the SIM schedule so the mirrored bag changes tick over tick.
The count accumulates to 8 over roughly 60 steps and then freezes flat —
`[(0, 8), (40, 8), (80, 8), (120, 8), (160, 8), (200, 8), (240, 8)]` — while
`sim.step()` keeps returning. ⇒ A clean `session_health` over those 240 frames
would have been a pass over a world that was not advancing, so the arm asserts
its own premise and is `#[ignore]`d as a probe rather than reporting green.

⛔ **AND THE PREMISE THAT CAUGHT IT WAS THE SECOND ONE I WROTE.** The first
asked `count > 0`, which the STARTER BAG satisfies on its own — it would have
passed without the system ever running. Two samples, with the later required to
exceed the earlier, is what turned "the value is nonzero" into "my system ran".

⇒ Next thing to try: add the system through `Platformer2dSimHarness::build`'s
`compose` callback, BEFORE the first update, rather than after the harness has
built and started its GGRS session.

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
  * `OwnedItems` via `dispatch_menu_action` → `dispatch_item_confirm` →
    `apply_menu_action`, which spells the write `owned.take(Item::HealthCell, 1)`
    — an equip or a consumable USE.

⛔ **AND THE REAL PATH DECREMENTS, SO THE REWIND HANDS THE ITEM BACK.** The arm
below grants, because an increment is the easier thing to observe; the shipped
menu `take`s. A rewind restores the pre-use count, so the health cell the player
just drank returns to the bag.

⚠ **I FIRST WROTE THAT THIS MAKES ITEM DUPLICATION THE LIKELY SYMPTOM. CHECKED,
AND IT IS NOT.** Duplication needs the HEAL to survive while the ITEM comes
back, and the heal does not: `apply_menu_action` writes `PlayerHealRequested`,
which
`crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs:622`
registers `clear_message_on_rollback`, so
the rewind clears the message as it restores the count. Both halves are undone
together. ⇒ The symptom is the quieter one — **the menu action silently does
nothing**, occasionally, only in netplay, and the state stays self-consistent
throughout. That is harder to notice and much harder to report, which is the
argument for fixing it rather than for relaxing about it.

⭐ **AND THAT MESSAGE IS THE FIX ALREADY BUILT.** Somebody made
`PlayerHealRequested` rollback-aware on this exact road. The item count beside it
was left as a direct write, so half of one action is rollback-correct and half is
not.

⭐ **THE TWO TYPES FAIL DIFFERENTLY, AND THE LOUDER ONE IS THE LUCKIER ONE.**
Both are written from a LOCAL menu, so only one peer makes the write; what
happens next depends on the registration kind, which
`RollbackEntryKind::feeds_peer_checksum` decides.

| type | kind | feeds the peer checksum | measured behaviour |
| --- | --- | --- | --- |
| `NewGameResetRequested` | `ResourceCanonical` | **yes** | taken back by the rewind, SILENTLY — the room is never rebuilt |
| `OwnedItems` | `ResourceClone` | **no** | taken back by the rewind, SILENTLY — the item returns |

⛔✦ **I PREDICTED THE HASHED ONE WOULD BE THE LOUD ONE, AND IT IS NOT.** The
table above originally read "makes A's and B's checksums differ — a DETECTED
desync" for `NewGameResetRequested`, reasoning that a hashed type must produce a
disagreement. MEASURED: it behaves exactly like the unhashed one. The write is
erased before it can reach a snapshot that anyone compares, so being hashed buys
nothing — **a checksum cannot disagree about a value that was put back before it
was taken.** ⇒ Registration kind predicts whether a SURVIVING divergence is
caught; it says nothing about a write that does not survive.

⚠⚠ **AND ONE THING THESE ARMS CANNOT SHOW, so the row must not claim it.** The
sync-test harness is ONE peer replaying itself. A write erased identically on
every replay produces no mismatch to detect, so whether TWO peers would disagree
in the window before the erase is a question no single-peer harness can answer.
The LOCAL LOSS is measured for both types. The cross-peer divergence is NOT
measured, and the earlier version of this row asserted it.

⛔ **SO `OwnedItems` IS THE ONE TO WORRY ABOUT.** Its kind is documented as
"snapshotted but not hashed: a rewind restores them, no peer reads them", and
that is exactly why nothing would report it: the player equips an item, a
rollback restores the pre-equip value, and the item is simply back in the bag
with no error anywhere. ⚠ `OwnedItemsBaseline` IS registered
`rollback_resource_clone_checksum`, so a projection of this state is hashed —
whether that projection would catch this write is the question to settle, not an
assumption to inherit from the kind's reassuring detail string. ⇒ **ANSWERED
BELOW: it does not.** `session_health` was clean on every one of the 240 frames
in which the grant was being taken back, so the hashed baseline does not stand in
for the unhashed value here.

⚠ **AND THE EXISTING TEST DOES NOT COVER IT, DELIBERATELY.**
`game/ambition_app/tests/rollback_full_reset.rs` asks whether the reset
RECONSTRUCTION is rollback-safe, and its own header says it folds a pending
request "into the baseline" so the work runs on the baseline frame and every
re-simulation of it. That is the safe shape by construction: a flag already true
before the sync-test window opens is identical on every peer and on every
replay. The mid-window menu write is the case nobody has asked about.

⭐⭐ **MEASURED 2026-09-16 — THE `OwnedItems` HALF IS REPRODUCED, WITH A
CONTROL.** `game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`
grants an item from outside the rewinding schedule, which is the shape
`dispatch_menu_action` makes when it equips, and drives the GGRS sync-test
window:

| harness | what happens to the grant |
| --- | --- |
| `with_sync_test_rollback_settings(4, 10)` | **GONE AT FRAME 0.** The live `OwnedItems` is back below the granted count on the very next step |
| same world, no rollback session | **KEPT for 240 frames** |

⛔ **SO THE REWIND TAKES IT BACK, AND NOTHING ANYWHERE SAYS SO.** No desync, no
error, no log line: `OwnedItems` is `rollback_resource_clone` — snapshotted and
restored, NOT hashed — so there is no checksum to disagree. The item is simply
back in the bag.

⚠ **AND THE SAVE MIRROR NEVER EVEN SAW IT.** `persist_inventory_to_save` is
value-compared, and the restore lands before it next runs, so it finds nothing
changed and early-returns. The autosave is therefore CONSISTENT with a world in
which the equip never happened — which is why no existing arm could have caught
this. `rollback_full_reset.rs` and `rollback_lifecycle_reset.rs` drive the same
mirror for 180 and 240 frames and are green, because in those worlds the bag
never changes.

⇒ The control is the load-bearing half of the arm. "The bag lost an item" and
"the REWIND took the item back" are indistinguishable from inside one harness.

⚠ **THIS IS NOT A GGRS BUG AND IT IS INVISIBLE IN SINGLE-PLAYER**, which is
between them why it survived. Restoring a snapshotted resource is exactly what a
rewind is for; the defect is that a player-visible ACTION is expressed as a
direct write to rollback state from outside the rewinding schedule. With no
session there is nothing to rewind and the control keeps the item forever — so
every hour of single-player play is evidence of nothing here.

⭐⭐ **AND THE FIX IS NOT A NEW PATTERN — `OwnedItems` ALREADY HAS A
ROLLBACK-CORRECT WRITE ROAD AND THE MENU DOES NOT USE IT.** MEASURED
2026-09-16:

  * `ItemGrantRequested` is registered `clear_message_on_rollback`
    (`crates/ambition_items/src/rollback_registration.rs`).
  * Its consumer `apply_item_grants` mutates `OwnedItems` and is registered into
    the SIM schedule (`features/mod.rs:214`), beside `apply_shop_transactions`
    and the effect-bus appliers.

⇒ So a conversation that gives you an item is rollback-correct today, and the
MENU giving you an item is not, for the same resource, in the same crate. ⚠ That
this guard has never flagged `apply_item_grants` is the cross-check: it reports
rollback mutators registered OUTSIDE the rewind, and that one is inside.

**Next implementation:** the `OwnedItems` half no longer needs investigating,
only fixing — have `dispatch_item_confirm` write `ItemGrantRequested` (and the
equivalent for a `take`) instead of mutating `OwnedItems` in place, which is the
road its own crate already ships.

⚠ **AND IT IS NOT A PURE REFACTOR, WHICH IS WHY THIS IS FILED RATHER THAN
DONE.** The menu READS `OwnedItems` in the same frame to render the row it just
changed. A deferred write means the sim applies the grant on the next tick, so
the list would show the old bag for one frame unless the UI is given something
to render optimistically. That is a visible behaviour change in shipped UI and a
maintainer's call, not a mechanical substitution. ⇒ The consuming direction already has a road too, and I nearly wrote here
that it did not: `ShopTransactionRequested` with `ShopSide::Sell` REMOVES from
the bag through `apply_shop_transactions`, in the sim, beside
`apply_item_grants`. ⭐ Its own doc states the rule this row is about, in the
engine's words rather than mine: *"a simulation system applies it on the tick
it was stamped for — every replay of that tick included."* A consumable USE is
not a shop sell, so the menu still needs its own message; what it does not need
is a new pattern. ⚠ The repro arm ASSERTS THE DEFECT so the lane stays green; when it
goes RED the defect is fixed, and the arm says so in place. Delete it and close
this row together.

⛔ **AND THE ESCAPE HATCH IS SHUT: THE RUN CONDITION GUARANTEES THE DANGEROUS
WINDOW RATHER THAN EXCLUDING IT.** The obvious hope is that a menu writing these
cannot be open while a session is live. These systems carry
`.run_if(simulation_authorized)`, and that predicate returns
`live_scope_of(..).is_some()` — it is TRUE exactly when a live session scope
exists. So they are gated to run only in the window that matters. ⚠ A live scope
is not by itself a live GGRS session (single-player has one too), but nothing
here narrows them to the single-player case.

**Still open:** only the cross-peer question, and it needs a TWO-PEER harness
rather than the sync test. Both local halves are now measured and both are
silent. ⇒ The fix does not wait on that answer: a local action that vanishes
some of the time is already a defect, and routing both writes through a message
the sim consumes fixes it whatever the answer turns out to be. If it cannot, this is two
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
**683 passed / 0 failed / 31 ignored**, 241.04 s at `c78cc725e` on the
ToothbrushAmbition box (2026-09-16, tree frozen for the run). Previously
677/0/25 at `582186bff` on the same box; the +6/+6 is the new arms and
print-only probes three agents added, and no arm changed state. Missing prerequisites are reported as incomplete rather
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

⭐ **`SimTick` ADVANCES 1:1 WITH `sim.step()` — MEASURED 2026-09-16, and it is
the discriminator nobody reaches for.** `fixed_60hz_room_sim("blink_run")`,
sampled every 40 steps: `[(0,0), (40,40), (80,80), (120,120), (160,160),
(200,200), (240,240)]`. A peer read a derived count freezing flat over 240 frames
as *"the simulation stops advancing ticks"*; it does not. ⇒ **"The sim stopped"
and "my writer stopped" produce identical evidence downstream, and only the TICK
tells them apart.** Sample `ambition_platformer2d::time::SimTick` before
attributing a frozen value to the schedule. ⚠ Scope: the fixed-tick harness. A
rollback composition is a different host in a different schedule, so re-measure
there rather than quoting this.

⭐ **AND `check_headless_arms_can_fail`'s 17 ARMS WERE AUDITED FOR THE INFLATION
THIS ROW'S OWN LOGIC INVITES — 0 EXPOSED (2026-09-16).** The check's rule is
"pins `ManualDuration` OR asserts something", which counts what an arm CONTAINS.
A peer warned that counting by what an arm CALLS rather than by what would FAIL
had inflated their own census six-to-zero. ⇒ Measured here instead of assumed:
10 of the 17 pass on asserts alone, and every one of them asserts something a
non-stepping engine cannot satisfy — a tick going 0 → 1, a counter reaching 60,
a life spent, a level clock advancing.

⚠ The two that looked like composition-only assertions (`a_fixed_aspect_profile_
reaches_the_camera_and_the_surround`, `an_undeclared_profile_leaves_the_host_
full_bleed`) POISON RED: removing the two `app.update()` calls from their shared
`presentation_shell` helper fails both, because `ResolvedGameplayPresentation` is
produced by those updates. ⚠ My first poison at those two removed zero calls —
they step through a helper, and `reachable()` expands it. A poison that edits the
wrong scope is a finding about the poison.

⭐⭐ **THE DISTINCTION IS WORTH MORE THAN THE RESULT: counting calls to a safety
API measures VIGILANCE; counting assertions a broken world fails measures
SAFETY.** An arm demanding a room change has a stronger liveness guarantee than
one reading a health API once at the end, because its check is load-bearing for
its own subject rather than bolted on beside it.

**Still open.** One non-reproducing session-root handoff failure whose assertion
message was never captured. ⚠ **IT DID NOT REPRODUCE AGAIN: 683/0/31 at
`c78cc725e`**, and the two arms it would have to be — 
`the_shipped_app_never_holds_two_session_roots_across_a_handoff` and
`a_candidate_session_replaced_while_pending_is_discarded`, both in
`an_edit_reaches_the_shipped_game.rs` — both passed, checked by NAME in the log
rather than inferred from the total. ⭐ That is now several clean full runs, and
a failure nobody can reproduce and nobody captured is not evidence of a defect;
it is an absent observation. ⇒ The next step is NOT more runs. It is that both
arms already assert their own premises (a root must appear; the activation id
must MOVE), so a future failure of either carries its cause in its message. On
the next reproduction, capture the full failing assertion and isolate the
production ordering/state source before changing test ordering or adding
retries. Keep compile-cost and prerequisite failures distinct
from behavioural flakes, and from CONTENTION — a coherent measured story that
fits the first observation is still the wrong one if it was never tested against
a second.

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.
