# The queue — live execution order

This file is the **current executable engineering queue**. It is not a work log,
review transcript or archive. Git history owns completed investigations. Durable
design and measurements belong in the linked owner document. Product decisions
belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

A row remains here only while an engineer can act on it. When it closes, keep a
short receipt only where another open row depends on that fact; otherwise remove
it.

Check a row's owner against who is actually running before you wait on them.

Growth is continuous and compression is per row, so one cleanup does not hold.
When a row closes, compress it IN PLACE to its receipt: what was wrong, what
fixed it, the commit, the guard and any standing prohibition. Move a reusable
rule to a page people read before they have the bug. Only a row's owner can
tell a receipt from live work. Measure the file with `wc -l` and the
per-campaign mass with

```sh
awk '/^### /{if(n)printf "%s %s\n", c, n; n=$2; c=0} {c++} END{printf "%s %s\n", c, n}' \
  docs/planning/queue.md | sort -rn | head
```

## P0 — architecture and correctness

### NEW-GAME-RESYNC — a New Game after durable hydration fails the sync test — ✅ DONE 2026-09-29

**Receipt.** A New Game is a checkpoint resume to the fresh baseline at the
start room, on the confirmed-frame road the death restore already takes:
`resume_at_checkpoint_on_reset` reads `NewGameRequested`, `AcceptedRestore`
carries `fresh`, and the fresh-run reducers run inside `CheckpointDomainApply`
(schema 262). ⛔ Do not register the four `derived` body types for rollback to
hide a rebuild on a speculative frame. Witness:
`a_new_game_asked_for_by_the_host_commits_once_under_a_rewind`, with
`rollback_health()` on both arms.

### SYNC-POINT-SENSITIVE-RESIM — a command sync point moves the death-reset replay

**Owner:** rollback determinism. **Found 2026-09-22** while moving the clock
into `SimClockHead`.

Adding ONE schedule edge — `ensure_sim_id → mint_spawned_sim_ids →
heal_projectile_owners` ordered `.before(SimClockHead)` — reddened
`rollback_lifecycle_reset::{a_player_death_reset_survives_the_rollback_window,
a_confirmed_death_restores_the_entitlement_bag_the_checkpoint_banked}` with a
GGRS sync-test mismatch, deterministically (three runs). The minting chain
touches no clock state; the only thing the edge changes is WHERE Bevy applies
that chain's `Commands`. `RollbackRestoreAudit` names the first diverging
frame's RESIMULATION rows: `SessionCheckpointOperations` one higher on replay,
`AcceptedCheckpointRestore` cleared on replay, `DeathInterlude`/`OutOfPlay`
present only on replay, then bodies. ⇒ Some system on the death-reset →
checkpoint road reads state whose value depends on command-application timing
and is not restored — the edge is not the defect, it is the probe that found
it. The edge was dropped (it had no reason to exist); the sensitivity remains.

⚠ **RE-MEASURED 2026-09-23 AT `5b9cabf10`: THE EDGE NO LONGER REDDENS ANYTHING,
AND THE AUDIT ROWS ABOVE WERE PARTLY THE AUDIT'S OWN DEFECT.** With the edge
re-applied, all five `rollback_lifecycle_reset` tests pass (the two death tests
run 2400 frames under `rollback_health` each frame). Separately,
`RollbackRestoreAudit` keyed its saved censuses by frame number ALONE, and a
lifecycle rebase restarts the frame count at zero — so the rebased timeline's
frame N was compared against the replaced timeline's frame N, and every
per-tick type (`SimTick`, `WorldTime`, bodies, checkpoint counters) read as a
resimulation divergence. Fixed: the audit drops its frame-keyed history when
`ConfirmedFrameBoundary::session` changes
(`a_rebased_timeline_is_not_compared_against_the_one_it_replaced`). With the
fix and the edge, a 400-frame audited death run reports no divergence.
⇒ The sync-test mismatch recorded above was real at the time (a GGRS checksum,
not the audit) and is not reproduced after `b7265b0ea`/`f2813e3e4` changed the
schedule and the player's abilities; whether those FIXED the sensitivity or only
moved the death off the frame that exposed it is not known. Kept open as a
lead, not a reproduced defect: the next probe is any edge that moves a sync
point on the death → checkpoint road, audited with the fixed instrument.

### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16

**Owner:** [construction and reconstitution](engine/construction-and-reconstitution.md),
which holds the audit, the witnesses and the acceptance record.

**Receipt.** A failed candidate world leaves world N playable and unchanged at
both scopes, and N+1 is verified before N is retired. Every road that changes
the authoritative world crosses its own publication verdict. Witnesses:
`game/ambition_app/tests/an_edit_reaches_the_shipped_game.rs` and
`death_restores_the_checkpoint.rs`. Post-A10 demolition closed 2026-09-16: every
A10 symbol has live production callers, and only `CandidateState` was dead
(deleted in `09629b060`). ⛔ Not part of A10: peer-stable identity (ID-PEER),
the defensive `DepartureAuthority::Custodian` fallback, and the refused-door
player signal.

### CUTSCENE-ROLLBACK-DECISION — two session-scoped cutscene values cross into simulation with no rollback decision — ✅ DONE 2026-09-28

**Receipt.** The cutscene dismiss and skip are seat intents on the
`ControlFrame` (`confirm_pressed`, `cancel_held`), which `tick_active_cutscene`
reads from `SlotControls`, and `CutsceneSkipHold` is rollback state
(`cutscene.skip_hold`, schema 258). The host-side `Update` request and its
producer are deleted. Guards:
`a_cutscene_dismiss_on_the_seats_input_survives_the_rewind_once` and
`a_cutscene_skip_is_a_hold_of_the_seats_cancel`
(`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`).

### ID-PEER — remove host-local lineage from peer-stable mechanical identity

**Owner:** deterministic identity / rollback architecture; see the identity map in
[`consolidation/architecture-census.md`](consolidation/architecture-census.md).

⛔⛤ **THIS BANNER HAS NOW SAID "COMPLETE FOR ITS CURRENT SCOPE" TWICE AND BEEN
WRONG BOTH TIMES, SO READ THE PATTERN RATHER THAN THE CLAIM.** On 2026-09-16 it
rested on a clean value census; on 2026-09-17 the review found a P0 the value
census structurally could not see — GGRS hashes a carrier ORDER the census does
not model — and deleting a FALLBACK the same day surfaced a second defect the
fallback had been covering. Both are closed. Three open roads have their decision
outside the campaign (`Q122`, `Q128`, netcode's `N2`). ⇒ **A pass over the TABLE
is still not work. What has twice been work is reading what the pinned dependency
ACTUALLY computes, and removing a substitute to see who was using it.** ⛔⛤ **AND A
THIRD TIME THE SAME DAY: the repair for that P0 was itself reviewed and found
CANDIDATE-BLIND** — its enumeration is an ordinary query and `InactiveCandidate`
is a disabling component, so the fix that closed the road had a hole the road's
own arms could not see, because their fixture never installs the filter. It
refuses now rather than rebasing a partial population. ⇒ **A road closed the same
day it was found has been reviewed once; the arms that hold it are the thing to
poison, not the conclusion.** ⛔⛤ **What has
twice been work is DELETING A FALLBACK AND RUNNING THE SUITE.** This banner said
"the census runs clean" on 2026-09-16 and on 2026-09-17 the refusal that replaced
perception's `Entity` fallback reddened two arms on the shipped route — a defect
the fallback had been covering, which no census over types or over frames could
see because it opens and closes INSIDE one frame. ⇒ The move that finds these is
removing the substitute, not re-reading the row.

⚠ **ONE EXCEPTION, NAMED BECAUSE A BLANKET "BLOCKED" WOULD SWALLOW IT.** The
float rows' STATE half is not blocked —
`two_local_histories_compute_the_same_mechanical_values` and
`two_local_histories_agree_about_the_sharp_unchecksummed_rows` are the road, and
between them they reach all TWELVE of S7's sharp rows, **ELEVEN of which carry
state and agree — measured 2026-09-17 over five walks (two of them driven),
sampled at every tick of a 121-step window rather than at four rungs.** ⚠ That is
twelve of the twenty-five: the remaining thirteen are the ones S7 does NOT rank
sharp. The twelfth was `gravity.flip_switch`, silent because NOTHING IN
PRODUCTION SPAWNED ITS COMPONENT — one construction site in the workspace,
inside a `#[cfg(test)]` module. ✅ `Q137` ruled on 2026-09-19 that gravity
switching stays and the unreachable overlap plate goes, so that row was DELETED
rather than covered: the walk reaches ELEVEN sharp rows and there is no twelfth
to reach.

⛔⛤ **AND THE PRICES THIS PARAGRAPH QUOTED FOR THE LAST THREE WERE ALL WRONG.**
Two of the three cost a room each and the third cost a maintainer question; the
rule that follows from it — *price a row by its INSERT SITE, not by the event it
is named for* — is a method and lives with the other methods, in
[`re-measuring-a-planning-claim.md`](../recipes/re-measuring-a-planning-claim.md#price-a-row-by-its-insert-site-not-by-the-event-it-is-named-for).

**Current state (2026-09-17): FIFTEEN CLOSED, THREE OPEN, EIGHTEEN LIVE —
nineteen FILED, because the thirteenth was withdrawn the day it was filed and the
numbering does not reuse it.** ⭐ **RE-DERIVE THAT FROM THE TABLE RATHER THAN
TRUSTING THIS SENTENCE:** the table below holds sixteen roads, thirteen closed
and three open, and the seventeenth and eighteenth are the two prose receipts
beneath it, both closed. 13 + 2 = fifteen, and all three open roads are in the
table.
⛔⛤ **A count in prose is a copy, and the copy nearest a correction is the one
that survives it** — this paragraph replaced four that each re-stated the count
in a different tense, the last of them still reading "eleven closed" beneath a
headline saying fourteen.

⛔ **ALL THREE OPEN ROADS ARE BLOCKED OUTSIDE THIS CAMPAIGN AGAIN, and the
fourth closed the day it was found.** Two want a maintainer decision before
anyone starts — the absolute `SimTick` (`Q128`, netcode) and the snapshot schema
fingerprint hashing English prose (`Q122`) — and the 25 unchecksummed float rows
are a DIFFERENT KIND of road: those carry no host-local id at all, they are
simply never compared between peers, so no projection can fix them and only
netcode's `N2` can observe them.

⚠ **AND ONE CLOSED ROAD CARRIES A HEDGE A READER SHOULD NOT LOSE.** The fifteenth
(`ControlFrame`'s shape) is closed for the RATCHET — a silent change is now
impossible — and NOT for a negotiated input version, which does not exist and is
not obviously owed while netcode is `N2`. If a P2P session is ever built, that
half returns as new work rather than as a correction to this count.

⛔⛤ **THE COUNT MOVED BY THREE IN ONE PASS, WHICH IS THE HONEST READING OF
"ID-PEER IS COMPLETE FOR ITS CURRENT SCOPE": IT WAS NOT.** The hostile two-host
peer-visible census (below) found three defects no type census could see, and the
instrument that says so now exists. ⭐ The third took two causes and one
refutation: the obvious explanation was measured, moved the number, and did not
close it.

⛔⛤ **A FOURTEENTH, FOUND AND CLOSED 2026-09-16: A LOCAL DEBUGGING INSTRUMENT
WAS AN INPUT TO THE PEER IDENTITY.** The same sandbox built default and with
`--features causal` produced two different `schema_fingerprint()` values from two
identical simulations, so both peers would compute the same checksums and then
refuse to play each other. Found `5967c98a7`, closed `190830022` with
`RollbackEntryKind::MessageClearInstrument` answering
`in_peer_schema_identity() == false`. ⇒ **The durable point is not that a feature
leaked — it is that the fingerprint and the dump filter DISAGREED about what
counts as schema, both deliberately, with nothing comparing them.** That, the
predicate, the arm and its positive control are owned by netcode's
[`N3`](engine/netcode.md#n3--contentschema-negotiation); this row does not
restate them.

⛔⛤ **A FIFTEENTH, FOUND 2026-09-16 BY VERIFYING A PRICE RATHER THAN A CLAIM:
NOTHING VERSIONED THE SHAPE OF THE PAYLOAD TWO PEERS EXCHANGE.** `ControlFrame`
IS the wire, and every candidate that looked like it covered this covers
something else.

✔ **THE RATCHET HALF IS LANDED** (`2bfa6e011`, `b1a380e63`, `224f65009`): the
exact bincode bytes of one legible frame are pinned, every frame is asserted to
encode to the same width, and the source census refuses a variable-width field
type at every level `ControlFrame` reaches. A change to the peer input payload is
now impossible to make silently. ⛔ **The CONTRACT, the GGRS reading it rests on,
what the bytes cannot see, and why a wire-identity bump buys a different
fixed-width protocol rather than a variable one are owned by
[`engine/netcode.md`](engine/netcode.md#the-input-payload-two-peers-exchange)** —
this row does not restate them, because it used to, at 162 lines, and a queue row
is not where a transport contract lives.

⇒ **STILL OPEN AND NOT OBVIOUSLY OWED:** a negotiated input version. Nothing
EXCHANGES the identity — the same remainder the state half has, waiting on `N2`.
⚠ `SETTINGS-ROLLBACK` is the row that would add the first new field and therefore
the row that had to notice; it is not blocked by this.
⚠ And the cutscene edge (`Q136`, [ruled](maintainer-decisions.md) 2026-09-19:
ingress by semantic ownership)
is the likelier first customer: if a cutscene edge is gameplay input,
this is the arm that will speak. ⭐ NARROWED 2026-09-18 to one named field: the
cutscene SKIP is a hold and `ControlFrame` carries `reset_pressed` without a
`reset_held`, so what that ruling would add is one bool beside the five `*_held`
fields already there — not a renegotiated payload. Touch is not an obstacle
either; it feeds the same `ActionState<Platformer2dInputActionMonolith>` and
binds `Reset` outright (`ambition_touch_input/src/virtual_device.rs:304`).

⛔⛤ **AND A THIRTEENTH WAS FILED THE SAME DAY AND WITHDRAWN WITHIN THE HOUR,
BECAUSE IT ALREADY HAD AN OWNER.** Walking the inputs of `possession_trigger_system`
found an App-local, menu-mutable USER PREFERENCE interpreting replayed stick input
inside the simulation — measured, real, and already waived with its reason in
`rollback_coverage.rs`, already owned by `SETTINGS-ROLLBACK`, and already carrying
a BETTER repair than the one I was about to propose. ⇒ A new row would have been a
second owner for one fact, which is the thing this campaign exists to remove. What
survived is this paragraph and a correction to that owner's stale "current state";
the table above is unchanged, because a cross-reference is not a road.

⭐ **WHAT THIS CAMPAIGN ADDS TO `SETTINGS-ROLLBACK` IS THE PEER HALF, WHICH THE
WAIVER DOES NOT STATE.** `rollback_coverage.rs` records the LOCAL consequence — a
resimulation of frame N interprets frame N's stick under whatever policy holds
now — and it records that registering the table as rollback state is the WRONG
repair, *"it is written from `Update`, which may not write rollback state"*. The
peer consequence is worse and needs no settings change at all to fire:
`holding_descend(control.axis_x, control.axis_y, gravity_dir, movement_mode)` in
`possession.rs` takes the AXES from GGRS's replayed input and the MODE from an
App-local preference, so **two peers with different accessibility settings
interpret the same exchanged input differently, from the first frame, forever, and
silently.** In that call site the mode decides which way "down" is for a
control-authority transfer.

⚠ The method lesson is narrow and worth keeping: **the row I nearly duplicated
says the frame-mode half is PROJECTED, and I read "projected" as "done"** — the
waiver eleven lines from the code says otherwise, and so does that row's own
acceptance criterion.

✔ **THE PEER-IDENTITY CHECKPOINT THAT C03 AND C05 WAIT ON IS DISCHARGED
(2026-09-16), AND THIS ROW IS ITS ONE OWNER.** `consolidation-plan.md` states the
gate's PURPOSE rather than a completion bar — *"finish A10 and peer identity first
so the live/candidate session owner is stable"* — so the discharge is a claim about
stability, not about this campaign being finished. It is not.

Every ID-PEER road that touches `SessionRoot`, activation or provenance is closed,
each with its arm named in the table below: the session root's canonical `SimId`
(now the constant `SimId::singleton("session", "root")`), the four
`MatchInstance`-stamped resources, `SessionMatchOrdinal`'s own registration and
its eager reset at `SessionScopeSet::Activate`, `MatchInstance::random_context`,
the checkpoint operation keys, and `TransactionId` provenance.

⛔⛤ **AND THE LAST OF THOSE WAS CLOSED AT THE PROJECTION AND OPEN AT THE
PRODUCTION ROADS FOR A DAY.** The guard that reported it closed —
`two_hosts_at_different_content_epochs_share_one_construction_provenance` —
asserts two hosts holding the SAME content agree, and `content-unstated` agrees
with `content-unstated` perfectly, so three ordinary roads dropping the content
term made that assertion MORE true. ⇒ The sixteenth road is taken:
`ContentBinding::canonical_summary` states the term, and poisoning it to a
constant leaves all six `id_peer_audit` arms green while the one arm that takes a
process through two prepared fingerprints and asserts the provenance MOVED
reddens. ⚠ One process at two fingerprints is not two peers, and is stated that
way; two hosts agreeing still needs `N2`. **The method rule this produced now
lives in
[`decision-principles.md`](decision-principles.md#an-equality-assertion-fa--fb-is-satisfied-by-every-f-that-throws-information-away),
which is its owner**; this row keeps the instance.

⭐ **AND THE STRONGEST EVIDENCE IS A COMMITMENT RATHER THAN AN ABSENCE.**
`TransactionId`'s closure did NOT remove the session stamp: the rendered stamp
still spells `{binding}\t{room}\t{session}` and MUST, because the construction
scope's gather filter and A10's candidate-vs-live separation read it — only the
peer PROJECTION drops the app-local epoch and the session term. So ID-PEER has
already committed in code to not changing the thing C03 depends on, and
ToothbrushAmbition's `a_superseded_transaction_cannot_publish_in_the_shipped_app`
now asserts that identity survives a supersession in the shipped composition.

⛔ **THE RE-ARM CONDITION, NAMED RATHER THAN LEFT IMPLICIT.** All three open roads
are blocked on something outside this campaign, so none is in flight — but one of
them would enter C03's neighbourhood if it ever started. **`Q128` rebases the
simulation tick "when peers agree to start", which is an ACTIVATION moment.** ⇒ If
`Q128` is ruled and started while a C03 or C05 migration is in flight, this
checkpoint re-arms and the two campaigns must coordinate rather than assume. The
other two (`Q122`'s schema-fingerprint prose, the 25 unchecksummed float rows)
cannot touch session ownership at all.

⚠ **WHAT THIS DISCHARGE IS NOT.** It is not a claim that ID-PEER is done — eleven
of fourteen roads, three open — and it is not a review of C03's or C05's own plans. It
says the identity neighbourhood they were told to wait for has stopped moving and
is pinned by arms.

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
| the peer-agreed match ordinal | **CLOSED** (schema 187) — `SessionMatchOrdinal` mints which match of the session it is; both the item draw context and `SimId::match_spawn` moved off the absolute activation tick. ✔ **AND `match_spawn` IS CLOSED BY SHAPE RATHER THAN BY VALUE, WITH NO PRODUCTION CUSTOMER — measured 2026-09-16 while looking for the value-level arm it appeared to owe.** Both arguments are match-relative by TYPE: `ActiveMatch::ordinal()` is the session's match ordinal, and `LiveMatchTicks::crossed` reads `micros`, which `advance_live_match_clock` zeroes whenever the match instance changes and which refuses a clock belonging to another match. `item_spawns` is `None` in the only production roster (`game/ambition_demo_smash/src/lib.rs:243` — a product decision, not a gap: *"we don't need items in smash right now"*) and nothing sets it to `Some`. ⇒ A value-level census over a shipped smash match, booted 30 updates against 300 and then run 240, reaches exactly THREE identities in both and they agree: `placement:smash_duelist_a#seat0`, `placement:smash_duelist_b#seat1`, `session:root`. No `match:*/spawn/*` row exists to compare, so an arm there would have to author the roster and would then assert what the types guarantee |
| the four `MatchInstance`-stamped resources | **CLOSED** (schema 190), after being closed WRONG **THREE** times. 186 moved them onto the activation tick and called it peer-stable. 187's correction then excluded the instance ENTIRELY, which was false-NEGATIVE: a verdict for the previous match checksummed identically to one for the live match while `settled(active)` disagreed. 190 gave `MatchInstance` a LOCAL half (staleness, `belongs_to`) and a PEER half (the ordinal) — correct WITHIN one session, and the GPT review of 2026-09-16 found the third hole: the ordinal restarts at zero every session, so `session A / match 0` and `session B / match 0` project identically while the three stale-tolerant resources are App-global and can hold A's stamp beside B's first match. ⇒ Closed 2026-09-16 by making the ANTECEDENT impossible rather than the checksum longer: the three stale-tolerant resources and the ordinal mint are members of `SessionScopedResources` now, reset at `SessionScopeSet::Activate`. Held by `a_match_stamp_from_the_previous_session_cannot_reach_the_next_ones_first_match`. ⛔⛤ **AND THE FIRST VERSION OF THAT CLOSE RESET THE MIRRORS AND LEFT THE AUTHORITY** — a second GPT review the same day found `ActiveMatch` itself surviving the boundary that resets everything stamped against it. Its own peer projection is `(seat count, ordinal)`, both written by the PREVIOUS session, so two hosts entering one new session after different histories (`match 0 / 2 seats` and `match 3 / 4 seats`) began it with different checksummed state — and mechanically observable state, since `count_the_live_match_ticks` reads any receipt as a live match. It is now REMOVED (not defaulted: there is no default live match) at the same edge, held by `two_hosts_with_different_prior_match_histories_enter_a_session_with_the_same_peer_state`. The versus/smash `releasing_witnessed` cleanup is a DIFFERENT boundary — a gameplay scope can change without leaving the shell route — and does not stand in for it |
| `SessionMatchOrdinal`'s own registration | **CLOSED** (schema 189) — it was `rollback_resource_canonical`, whole-value, with a comment beside it claiming the `session` half "is compared only against ITSELF". The sentence described `take`; the registrar decided the checksum. ⚠ The lazy-reset window that was RECORDED here is closed as of 2026-09-16, by the same edge as the row above: the mint is reset eagerly at `SessionScopeSet::Activate`, so it cannot carry the previous session's count into this session's first activation. `take`'s own check survives as the answer for a composition that has only one session |
| `MatchInstance::random_context` | **CLOSED** — the method moved to `ActiveMatch` and reads the ordinal. This row said OPEN while the row above said CLOSED, which the review flagged as contradictory control-plane text |
| checkpoint operation keys | **CLOSED** (schema 188) — the peer projection is the ADMISSION SEQUENCE plus whether a scope owns the operation; the scope keeps its stale-operation job and still round-trips, because all three carriers snapshot by `Clone` |
| **the session root's canonical `SimId`** | **CLOSED 2026-09-16** — it was `SimId::singleton("session", activation_id)` on BOTH mints, and `ShellActivationId` is a per-App route count inside a `component-canonical` comparison. ⭐ The count was disambiguating NOTHING: a canonical identity only needs to be unique inside the world a checksum compares, and `shell_host_lifecycle` already pins `session_roots == 1` in game and `== 0` at home across a four-session lifecycle, rollback variant included. Both mints are `SimId::singleton("session", "root")`. ⛔⛤ **And the arm that was cited for it held the road production does not take** — `spawn_world_for` has no production caller; A10's candidate road builds its own root and hands it to `adopt_world`. Poisoning each mint separately (2026-09-16): the candidate poison left the pre-existing app suite green at **705 passed / 0 failed**. Held on the shipped road by `two_local_histories_name_every_simulated_entity_identically` (`shell_host_lifecycle`), which censuses all 22 canonical identities in a built world. ⭐⭐ And there is ONE mint now: the unreachable primitive is deleted, and the arm that certified this class through it is retired with it. See below |
| `TransactionId` provenance | **CLOSED 2026-09-16 at both ends, having been closed at only one for a day.** The projection half (schema 193) was the campaign's original finding: the stamp still renders `{binding}\t{room}\t{session}` and MUST, because the construction scope's gather filter and A10's candidate-vs-live separation read it, while the peer projection keeps the content identity and the room and drops the app-local epoch and the session stamp. It is the first COMPONENT to state a projection, which needed `rollback_component_canonical_checksum` to exist. ⛔⛤ **But the GPT review found the term the projection KEEPS was ABSENT on three of the four roads that mint one.** `ActorConstructionContext::for_room_construction` <!-- cite-ok: the removed signature is what this row records --> took `content` and `active_binding` separately and applied the second to the expected-live half only, so the door transition, the reset and the neighbour prefetch each answered `content_unstated` for the INCOMING half — reasoning correctly that a transition publishes no content, which is a fact about the commit boundary and not about provenance. MEASURED: after one door transition the only peer content term anywhere in the live world was `content-unstated`, so two peers at different prepared content projected identically. ⇒ Repaired as a SHAPE: `for_live_room_construction` takes ONE binding and the split is unspellable, `for_content_replacement` takes two by name, and only a hot reload asks for it. Held by `an_ordinary_room_transition_stamps_its_roots_with_the_session_content` plus the provenance half of the death and reset arms beside it (`5bb3cc8ea`) |
| **the snapshot schema fingerprint** | ⛔ **OPEN, AND BLOCKED ON A MAINTAINER — `Q122`.** `schema_dump()` emits a prose `detail` per row and `compute_schema_fingerprint` hashes the whole dump, so English wording is inside the identity `ActiveRollbackAuthority::installed` gives a timeline. Measured by poison: pluralising ONE WORD in `detail::MESSAGE_CLEAR` turns the baseline red with 166 diff lines, 83 added and 83 removed. That is host-local lineage in a peer-stable identity in its purest form — two builds of the SAME mechanical schema are two identities if somebody reworded a comment. ⚠ The naive fix is refuted: of 493 rows, 268 carry facts `kind` does not encode (entity handle vs SET vs keyed MAP remapping, identical vs presence-aware canonical checksums, 22 custom-checksum descriptions), so dropping `detail` would stop the fingerprint seeing an entity-remapping change. The shape is a split, and where the line falls is the decision. ⇒ Landed meanwhile without needing it: the 15 sentences had TWO owners across two crates with nothing comparing them, and now have one (`879a5a1a3`, dump byte-identical). ⭐⭐ **AND A SECOND, 2026-09-17, WHICH THE DUMP COULD NOT SEE BECAUSE BOTH SPELLINGS WERE RECORDED IN IT:** five portal message types (`ClearPortals`, `DropPortalGun`, `FirePortalGun`, `PickUpPortalGun`, `TogglePortalGun`) each carried TWO rows — a canonical name and a historical alias kept *"so the full compatibility registration keeps the rollback schema byte-for-byte"* — and nothing outside the definition and the baseline ever read the alias. ⚠ It was not only a duplicated row: `should_install_backend` dedupes on the registration's stable NAME rather than its type, so each alias also installed a second `clear_message_channel::<T>` into `LoadWorld::Mapping` — ten systems doing five jobs, excused by name in `boot_budget.rs`'s deliberate-duplicate list. Removed; **493 rows → 488**, schema v196 → v197, the five exemptions deleted in the same commit because a stale excuse is a hole with a comment over it. ⚠ **488 IS THAT COMMIT'S NUMBER, NOT TODAY'S, AND NEITHER IS 491** — `Q142` added three rows on 2026-09-17 (v197 → v198, baseline 491) and `Q137` deleted `GravityFlipSwitch`'s two on 2026-09-19 (v199 → v200, baseline **489**); re-derive with `tail -n +2 game/ambition_app/tests/rollback_schema_baseline.txt | wc -l` rather than reading any figure on a page as current. ⇒ Held by `no_two_schema_rows_describe_the_same_type_the_same_way`, stated over (TYPE, KIND) because one type legitimately holds rows of DIFFERENT kinds and two rows of the same kind say one thing twice |
| **the accumulating gameplay clock** | ✔ **CLOSED 2026-09-16, FOUND BY THE TWO-HOST PEER-VISIBLE CENSUS.** `GameplayElapsed(f32)` has ONE writer — `advance_gameplay_elapsed`, `+= scaled_dt` every frame — is `init_resource`'d at App build, and was reset nowhere, while registered `rollback_resource_canonical` so its WHOLE value is compared between peers. Two hosts that reached the same route by different shell histories disagreed about it on the frame they arrived. ⭐ **UNLIKE `SimTick` IT NEEDED NO RULING, WHICH IS THE WHOLE DIFFERENCE**: `Q128` is open because a projection excluding the tick would exclude the TIMELINE, and this is a lookback clock whose only consumer is the brain's reaction-latency window (`actors/update.rs`), which a session-relative clock answers identically. ⇒ Added to `SessionScopedResources`, reset at `SessionScopeSet::Activate` — the group that already existed for exactly this. Held by `the_peer_visible_surface_does_not_record_which_route_the_host_visited_first` |
| **the startup-resume checksum** | ✔ **CLOSED 2026-09-16, SAME CENSUS.** `SessionStartupResume::checksum` — the projection handed to `rollback_resource_clone_checksum`, i.e. the function peers compare — hashed the session generation itself, and that generation is `SessionScopeId.0` (`restore_checkpoint_on_session_start`: `let generation = scope_id.map(\|id\| id.0)`). Measured `4354685564936845353` against `4354685564936845357`, scope `0` against scope `2`. ⇒ The projection now tags the generation's PRESENCE and drops its value. ⭐ **THAT IS ONLY SAFE BECAUSE THE ANTECEDENT IS ALREADY SHUT**, which is the `MatchInstance` lesson applied rather than repeated: excluding a local stamp with nothing identifying WHICH session the value describes is false-NEGATIVE, and `reset_checkpoint_coordinator_on_activation` already defaults this resource at the activation edge, so a foreign generation cannot be alive to compare. The generation STAYS in the value — `state_for` filters on it |
| **an ECS entity index in a peer checksum** | ✔ **CLOSED 2026-09-16, AND IT TOOK TWO CAUSES AND ONE REFUTATION TO GET THERE.** `PerceptionMemory` differed between two hosts whose only difference was which routes they visited first. ⛔⛤ `GameplayElapsed` was the obvious cause — perception is handed that clock and `RememberedActor::last_seen` is *"sim time the actor was last directly in view"* — and **fixing the clock MOVED the row without equalising it** (veteran `10957388069613372399` → `5036184031634534872` while fresh held). A mechanism that explains the number is not evidence for it. ⇒ The second cause was `collect_perception_peers`, whose id fell back to `format!("e{}", entity.index())` for a body with no `FeatureId`. That string is the KEY of `WorldMemory::actors`, a `BTreeMap` inside a `rollback_component_canonical` component — so a Bevy allocation-order artefact was compared between peers. **Measured: the same route perceives the player as `e888` in one host and `e1026` in the other.** ⚠ AND THE CHECKSUM IS THE SMALLER HALF: `WorldMemory`'s own doc says it is a `BTreeMap` because *"`last_known_hostile` takes the `max_by` confidence over these… so the tie is broken by iteration order"*, so two peers with index-built keys order their memory differently and an NPC with two equally-confident targets chases a different one on each. ⇒ The fallback is now `SimId`, which the body already carries; `FeatureId` stays first because that is what hostility and targeting look bodies up by. Held by the hostile arm, poison-verified in both directions. ⭐⭐ **AND DELETING THE FALLBACK FOUND A SECOND, OLDER DEFECT THE FALLBACK HAD BEEN COVERING — 2026-09-17.** The road refuses now (`error!`, `debug_assert!`, `continue`) rather than naming a body by its index, and the first full app run at that tree fired the refusal on the SHIPPED Ambition route in the only TWO arms that re-enter a live route — `id_peer_audit::two_hosts_with_different_local_history_seat_the_same_agreed_match` and `edit_to_play_through_the_shell::an_edit_on_disk_reaches_the_constructed_actor_through_the_shell` — one body each, `faction=Player primary=true player_entity=true scoped=SessionScopeId(1)`. ⚠ Both were a NEW red, not the intermittent this suite already knows about: the two failures carry the same panic at the same line, which is what said they had one cause. ⇒ The cause is an ORDERING window, not a missing mint: `ensure_sim_id` backfills `SimId::player_slot(0)` for a `PrimaryPlayer` body at the HEAD of the sim, so a player spawned into a re-entered route AFTER that system ran carried no canonical identity for the rest of its first frame — and `collect_perception_peers` reaches it in that frame. Before the refusal, perception simply remembered the player under its allocation index, which is exactly the artefact this row is about. ⚠ **A CENSUS BETWEEN FRAMES CANNOT SEE IT AND SAID SO:** the same fixture, inspected after every `step`, reports ZERO unnameable perceivable bodies at every step — the window opens and closes inside one frame, and only an instrument INSIDE the frame (the refusal itself) reports it. ⇒ Closed where the rule already said it belonged — *"its spawn site must mint an identity"*: `PlayerIdentityBundle` mints `SimId::player_slot(slot)` at construction, keyed on the bundle's OWN slot, so a second local player gets `slot:1` where the backfill would have given it `slot:0`. `ensure_sim_id`'s `PrimaryPlayer` arm is the net for a body that BECOMES primary later, and says so. The arm was RED at `2f05ec51f` and green after, which is the controlled pair, and the full app suite is **711 passed / 0 failed / 45 ignored** at the fix (it was 709/2 at the tree that found it). ⚠ **WHAT THE REFUSAL PINS IS NOW WIDER THAN THE PLAYER:** `ensure_sim_id` sweeps at the HEAD of the frame and again at the TAIL, and perception runs between them, so the assert is a standing claim that NOTHING reaches perception unnameable — mid-frame arrivals included. `UnmintedBodyCensus` cannot make that claim and does not try: it skips `Added<BodyKinematics>` this tick and treats `PrimaryPlayer` as nameable, both deliberately |
| **the 25 unchecksummed float rows** | ⛔ **OPEN, AND NOT ANSWERABLE IN THIS WORKSPACE.** Not a lineage road like the ten above — these carry no host-local id; they are simply never compared between peers. **S7** in [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md) ranks them: of the 99 rows outside the session checksum, 25 are also read by an unfiltered per-tick query AND carry a float-bearing field, and 12 of those are mutably written in production. ⚠ **THE COUNT OF WHAT IS MEASURED IS FURTHER DOWN THIS ROW, and this sentence used to carry a stale copy of it** (*"two are measured clean"*, while the same row said twelve of twelve fourteen lines later). What does not move: `Session::SyncTest` is the only session this workspace constructs, so a local resimulation clears a row of a RESTORE defect and says nothing about two peers. ⇒ The TIMELINE half's blocker is N2's absent P2P session, the same blocker `Q128` has. ⭐⭐ **THE STATE HALF IS NOT BLOCKED, AND SAYING IT WAS COST THIS ROW A ROAD — CORRECTED 2026-09-16.** No P2P SESSION can be built; two APPS WITH DIFFERENT LOCAL HISTORIES can. `two_local_histories_compute_the_same_mechanical_values` (`game/ambition_app/tests/shell_host_lifecycle.rs`) launches the shipped Ambition route first in one host and third in another (scope `0`/epoch `1` vs scope `2`/epoch `3`, asserted first), then compares `BodyAnimFacts` BITWISE by canonical `SimId` across 120 steps — and the ground items by construction, labelled as such because they are measured AT REST in that route. It agrees. ⚠ It does not retire N2: no transport, no input exchange, no interleaving, no rebase. ⛔ Its first version was VACUOUS — two `Platformer2dSimHarness` instances in one process both read `SessionScopeId(0)` at tick 1, because a fresh App is a fresh counter, so the differing history has to live inside ONE App that has been somewhere first. ⭐⭐ **AND THE STATE HALF NOW COVERS ALL TWELVE SHARP ROWS RATHER THAN TWO, WITH ELEVEN OF THEM ACTUALLY CARRYING STATE — MEASURED 2026-09-17.** `two_local_histories_agree_about_the_sharp_unchecksummed_rows` (same file) takes the COMPLEMENT of the peer-visible arm's join: it asserts each of S7's twelve mutably-written float rows does NOT feed the peer checksum (so the two arms cannot drift into reading the same surface), then compares the probe census of those rows across the two hosts at EVERY tick through 120, in each of FIVE ROOMS. **12 of 12 registered, 11 carried state and agree** — `actor.animation_facts`, `actor.render_size`, `entity.transform`, `feature.hazard`, `item.ground_item`, `player.blink_camera_state`, `portal.emission`, `portal.gun_pickup`, `portal.placed`, `portal.shot`, `boss.death_animation` — and that eleven is pinned by SET EQUALITY, so a row losing its carriers reddens rather than passing quietly. ⛔⛤ **THE SIX-TO-NINE MOVE WAS A ROOM AND A PRESS, NOT A FIXTURE.** This row read the six silent rows as *"a fact about the route"*; for two of them it was a fact about the ROOM. `portal_lab` authors fourteen `Portal` placements and `basement_hazards` three `DamageVolume`s, so pinning the start room with `StartRoomOverride` + `StartRoomMustResolve` covered `portal.placed` and `feature.hazard` with no input road and no simulation change. ⚠ The first aim missed: `basement_npcs`'s `HazardBlock` is a SURFACE, not a `PlacementSchema::Hazard` — `DamageVolume` is the identifier that becomes `feature.hazard`. ⭐⭐ **THE NINTH IS THE ONE THING A ROOM CANNOT AUTHOR — A PRESS.** A fourth walk starts in `portal_bridge` (player x=94, authored `PortalGunSpawn` x=180, 20px half-extent) and drives `axis_x: 1.0` with an attack EDGE every ten steps through the shipped `drive_control_frame`; the gun is in hand on step 20 and `portal.shot` has carriers on both hosts thereafter, agreeing. Both hosts get the identical script — a pure function of the step index — so they still differ in exactly one thing. ⛔ The anti-vacuity floor is on the INTERSECTION: poison-verified both ways — one row's value perturbed reddens it naming the row, and misspelling every row name trips the floor with *"the registry spells 0 of the 12"* instead of passing over nothing; the new coverage set and the strict start-room resolve are poison-verified too (dropping a room names the row it lost; a mistyped room id panics with the valid list instead of booting elsewhere). ⇒ **THE AUTHORED HALF OF THE REMAINING WORK IS SPENT, AND THE CHEAP DRIVEN HALF WITH IT.** Of the three still silent, `gravity.flip_switch` could be placed by NO route — its only mutable writer was registered exactly once in the workspace and that registration was inside a `#[cfg(test)]` module, so S7 ranked a writer no production composition installs and the honest count of reachable sharp rows is ELEVEN. ✅ **RULED AND DELETED 2026-09-19 (`Q137`):** gravity switching stays and the unreachable overlap-plate vertical went — the component, its system, both rollback registrations inside the schema fingerprint, the per-tick view rebuild over an always-empty query and the render sync that rebuilt from it every frame, all of it duplicating a mechanic the shipped encounter `Switch` already owns. The row is gone, so eleven is the whole population rather than eleven of twelve. ⛔⛤ **`portal.emission` WAS FILED HERE AS A THIRD AND THAT WAS WRONG IN BOTH HALVES — CORRECTED THE SAME DAY.** It was read as wanting an aimed script (fire at a reachable wall, then walk into it). It wanted no gun: `portal_lab` — already the second walk — AUTHORS the aperture (`a_purple`, a ground-ground pair at x 254..346, 174px to the player's right), so driving that walk reaches it at tick 50. What hid it was the OBSERVATION: `PortalEmission` lives 0.18 s ≈ 11 ticks and the ladder sampled 0/1/30/120, whose widest gap is 90. The arm now samples every tick, and both halves are poison-verified separately. ⛔⛤ **AND `boss.death_animation` WAS THE SECOND OF THE PAIR, ON THE SAME MISTAKE.** It was filed as wanting a boss death, "the most expensive fixture of the set". A boss does not have to die: `BossDeathAnimation::default()` is inserted AT SPAWN (`actor_spawn/mod.rs:1141`), there are ELEVEN authored `BossSpawn` placements across nine rooms, and `basement_boss` is now the fifth walk. ⚠ Its carrier is constant `(1, 0)` at all 121 observation points, so what is compared is PRESENCE and identity rather than a varying float — a real boss death remains the stronger observation and the expensive fixture. ⇒ NO row is left that a route does not reach, and none is left waiting either: the last one, `gravity.flip_switch`, was re-checked 2026-09-17 with the same insert-site lens that overturned the two rows beside it — one construction site in the workspace, inside a `#[cfg(test)]` module opening thirty-three lines above it — and the ruling that followed deleted it. | <!-- cite-ok: `PortalGunSpawn` is an authored LDtk entity identifier (`ldtk_entity_contract.json`), not a Rust definition; the Rust name is `PortalGunSpawnSpec` -->
| the canonical timeline itself | ⛔ **OPEN, AND BLOCKED ON A MAINTAINER — `Q128`** in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). The absolute `SimTick` is `resource-canonical`, so two Apps running for different lengths of time disagree from the first compared frame. It cannot be closed the way the other nine were: a projection excluding the tick would exclude the TIMELINE, which is what a rollback comparison is about. It needs a session-relative tick rebased when peers agree to start, and where that agreement comes from is netcode. See below |
| **the GGRS carrier ORDER inside the component checksum** | ✔ **CLOSED 2026-09-17, THE SAME DAY IT WAS FOUND — AND IT IS THE ONLY ROAD HERE THAT NEEDED NO RULING.** `ComponentChecksumPlugin` hashes `RollbackOrdered.order(rollback_id)` together with the value projection before XORing carriers, so the checksum two peers compare contained an App-lifetime INSERTION INDEX. `RollbackOrdered` keeps every index it ever handed out, deleted entities included, is itself snapshotted, and nothing rebased it: `install_rebased_sync_test_session` reset `RollbackFrameCount`, `ConfirmedFrameCount`, the input authority and `Time<GgrsTime>`, and deliberately retained the snapshot infrastructure. ⭐ **MEASURED on the two-host fixture:** the same 22 canonical identities at orders `0..21` against `74..95` — a CONSTANT offset of 74, the rollback entities Sanic and Mary-O registered and retired — and **59 of the 146 real `ChecksumPart`s disagreed**, `BodyHealth`, `ActorPose`, `Brain` and `WornCharacter` among them. ⛔⛤ **AND THIS IS WHY THE VALUE CENSUS SAID CLEAN**: the probe folds `count` and a wrapping sum of the per-value projection and ignores WHICH ENTITY carried each value, so it measures GGRS's projection and not GGRS's checksum — `decdb036b`'s instrument fix was necessary and not sufficient. ⇒ Closed by `rebase_rollback_carrier_order`, called where the session declares frame zero, beside the counters it already rebases. **59 → 2**, and both survivors are open roads somebody else owns: `SimTick` (`Q128`) and `AmbitionGameSave` (`Q129`), each named in the arm with its reading and each asserted to STILL differ, so a spent excuse cannot sit there covering the next type. ⚠ It keys on canonical `SimId`, never on `RollbackId` — that is the Bevy `Entity` that first received `Rollback`, so ordering by it would swap insertion history for allocation order, the same defect one layer down. The previous order is the TIE-BREAK. ⛔⛤ **THE REASON THIS ROW GAVE FOR THAT TIE-BREAK WAS FALSE AND IS WORTH KEEPING AS A LESSON** — *"a candidate world and the live world may legitimately hold one canonical identity twice"* is true of the WORLD and false of THIS QUERY, which cannot see a candidate at all. Corrected 2026-09-17 at the code and here; the tie-break is what makes the sort total on a population that should hold no duplicate, and `two_carriers_sharing_one_sim_id_keep_this_app_s_spawn_order` now measures what it actually does: a total order that reproduces the LOCAL spawn order, stable between a fresh App and a veteran one because `previous order` is monotonic in spawn order and not because either key is peer-stable. Poison-verified with `u64::MAX - previous_order`, which inverts the duplicate pair and leaves every distinct identity where it was. ⚠ **`RollbackOrdered::push` is private upstream and there is no rebase API**, so the ordering is rebuilt by removing `RollbackId` and `Rollback` and re-adding `Rollback`, whose `on_add` hook mints `RollbackId::new(entity)` — the SAME value for the same entity — and pushes. A small upstream primitive would be better; this is what exists. Held by `two_local_histories_compute_the_same_ggrs_component_checksums`, poison-verified: dropping the rebase call reddens it at 57 of 146, which is the 59 less the two waived. ⛔⛤ **AND THE REPAIR REDDENED AN ARM THAT WAS NOT ABOUT ROLLBACK AT ALL, WHICH IS THE OTHER FINDING.** `a_fighters_percent_and_policy_survive_a_rewind` reported *"went into the rewind at 188% and came out at 0%"* — about a rollback that had worked. The roster seats TWO fighters; the arm damaged `.next()`'s and then read `.next()`'s again, and moving entities between archetypes flipped Bevy's iteration order. **Measured: the damaged fighter still held 188% and the arm was reading the other seat at 0%.** ⇒ An arm about rollback must not identify its subject the way rollback forbids; it pins the `Entity` now. ⭐ Two mechanisms for that red were proposed and MEASURED FALSE first — a re-minted `RollbackId` (every id still came from its own entity: 0 mismatches at all four rebases) and the resource replacement (keeping the old resource reddened it identically) ⛔⛤ **AND A SECOND REVIEW FOUND THE ENUMERATION IS CANDIDATE-BLIND, 2026-09-17.** `InactiveCandidate` is a registered DISABLING component, so the rebase's ordinary `With<Rollback>` query skips every candidate root; rebasing over the visible half leaves a hidden carrier holding a `RollbackId` absent from the rebuilt table, and `RollbackOrdered::order` PANICS for an id it does not know. ⛔ INCLUDING them would be worse — an order is an INDEX, and an index is positional, so a candidate present on one peer and absent on the other shifts every order after it, which is this row's own defect re-entered through the front door. ⇒ It now counts through `ambition_platformer2d_shared_tangle::construction::count_matching_including_hidden_candidates` and REFUSES, leaving the old order intact. Held by `a_hidden_candidate_carrier_stops_the_rebase_instead_of_being_dropped`, which is the arm the others could not be: their fixture never registers the disabling filter, so none of them could tell a candidate-blind enumeration from a complete one. Poison-verified: with the refusal disabled the rebuilt table holds 1 entry where the world has 2. |

✔ **AND THE HOSTILE RUN FOUND AN INSTRUMENT DEFECT ALONGSIDE THE THREE CODE
ONES, WHICH IS WHY ITS FIRST NUMBER WAS NOT ITS ANSWER. CLOSED 2026-09-16
(`decdb036b`).** Three registration arms —
`rollback_component_canonical_checksum`, `rollback_resource_canonical_checksum`
and `rollback_resource_optional_canonical_checksum` — each hand a
`fn(&T) -> u64` to GGRS and then registered the diagnostic probe with
`census_state`: the whole canonical state, including the very local terms the
projection exists to drop. `census_with`'s docstring asserted the opposite of
its own callers — *"the registration arms that take `checksum: fn(&T) -> u64`
hand the same function to GGRS and to this"* — which was true of ONE of the four
such arms. So every peer question this campaign asked through the probe was
asked of the wrong function for every road ID-PEER projected.

⇒ **The repair was NOT to swap `census_state` for `census_with`, because a
RESTORE audit must compare whole state and that is what the probe is for.** Two
questions, two censuses: `ChecksumProbe` now carries an optional peer projection
(`with_peer`), the three arms attach the same closure they hand GGRS, and
`RollbackChecksumProbes::census_all_as_peers_compare` uses it where declared and
falls back to whole state where a registration compares whole state anyway.
`census_all` is unchanged and still answers the restore question. ⛔⛤ **AND THE
SECOND CENSUS ANSWERS THE PEER PROJECTION, NOT THE PEER CHECKSUM — THE DAY AFTER
THIS LANDED, A REVIEW SHOWED WHY THAT DISTINCTION IS NOT PEDANTRY.** GGRS hashes
the carrier ORDER beside each value and XORs the per-carrier hashes, where this
census counts and wrapping-sums projections, so it agreed on 146 of 146 rows
while the real checksum disagreed on 59. See the carrier-order row above; the
docstring on `census_with` now prints both folds side by side.

⭐ **`TransactionId` is the proof, and it was the standing waiver that proved the
instrument rather than the code.** Under the whole-state census its row differs
between the two hosts; under the projection GGRS actually compares it agrees.
The two-host arm no longer carries an `EXPECTED_TO_DIFFER` entry for it — it
compares the peer-projected census and asserts, via
`peer_projected_type_names()`, that `TransactionId` is IN the projected set, so
dropping the `with_peer` attachment reddens the arm instead of silently
restoring the over-report. Poison-verified in that direction.

⚠ **AND THE FIRST RUN OF THE CENSUS ITSELF REPORTED SEVEN DIFFERING ROWS OVER ALL
364 PROBES, WHICH IS NOT THE QUESTION.** `probes.rs` says so in its own words —
*"what makes an entry dangerous is that it ALSO feeds the peer checksum, which the
registry knows and this does not; the JOIN is the finding"* — beside a record of a
previous instrument that over-reported by the same factor and announced itself as
a discovery. Joining against `feeds_peer_checksum` narrows 364 probes to 145 and
seven rows to six (⚠ **145 is THAT RUN's number; the surface is 146 of the
baseline's 491 rows as of 2026-09-17**, after `Q142` added three — re-derive by
raising the arm's `keep.len() >= 140` floor until it fails, and `tail -n +2
game/ambition_app/tests/rollback_schema_baseline.txt | wc -l`): one was the probe artefact above, two were already owned
(`SimTick`/`Q128`, `AmbitionGameSave`/`Q129`), and the remaining three are the
three defects this pass closed — all in the table above.


✔ **THE SESSION ROOT'S IDENTITY WAS A HOST-LOCAL ROUTE COUNTER, AND IS NOT NOW.**
Found by the GPT review of 2026-09-15 and closed 2026-09-16. It was
`SimId::singleton("session", activation_id)`; it is
`SimId::singleton("session", "root")`, in the one place that still mints it.

⛔⛤ **AND FOR A DAY THAT WAS THE ONLY ARM, HOLDING A ROAD NOTHING SHIPS.** The
root has TWO mints, and this row said so — but it cited one arm for both.
`ActiveGameplaySession::spawn_world_for` has no production caller: A10's <!-- cite-ok: the deleted mint is what this row records -->
candidate road builds its own root and hands it to `adopt_world`, which
`PlatformerSessionBuilder::build_candidate` states in its own doc it must,
because the primitive validates against the already-published session and a
candidate deliberately is not that session yet. Measured by poisoning each mint
separately:

| poisoned mint | production reachable | the primitive's arm | pre-existing `app_it` (751) |
|---|---|---|---|
| `spawn_world_for` (deleted since) | **no caller** | FAILED | — | <!-- cite-ok: the row records the name that was deleted -->
| the A10 candidate mint | **yes, the only road** | passes | **705 passed / 0 failed** |

⇒ The road the game takes was unguarded, and the census that proves it now is
`two_local_histories_name_every_simulated_entity_identically`
(`shell_host_lifecycle`): launch Ambition first in one host and third in another,
after two other providers have come and gone, and compare every canonical `SimId`
in the built world. 22 rows, identical; scope `0` vs `2` and epoch `1` vs `3`
asserted DIFFERENT first, so the comparison is controlled. ⚠ An equality arm is
satisfied by the constant, so the disagreement half is a Sanic session: a
different 43 rows sharing exactly three — the session root, the player slot, and
the App-build encounter authority.

✔ **AND THE QUESTION WAS THEN ASKED OF EVERY OTHER ROW: ONE DEFECT IN NINE
ARMS.** If one cited arm held a road nothing ships, the others owe the same
check. Each of the ID-PEER agreement/disagreement arms was traced to what it
actually drives:

- on the shipped composition already —
  `an_ordinary_room_transition_stamps_its_roots_with_the_session_content` (a real
  door in `fixed_60hz_sim`, with a premise check, two anti-vacuity floors and the
  POSITIVE assertion that no stamp reads `content-unstated`),
  `a_superseded_transaction_cannot_publish_in_the_shipped_app`,
  `two_differently_aged_hosts_publish_the_same_roster_through_the_shipped_road`,
  `two_hosts_at_different_content_epochs_share_one_construction_provenance`,
  `a_different_agreed_configuration_draws_a_different_sequence`;
- subject is a pure function, so the function IS the road —
  `the_same_verdict_for_a_different_match_is_a_different_checksum`
  (`peer_stable_checksum`) and
  `a_transaction_stamp_depends_on_host_local_lineage_and_must_keep_doing_so`
  (`ConstructionScope::transaction`);
- hand-built inputs, checked against the production call —
  `two_activations_are_two_draw_contexts` passes `Some(0)`/`Some(1)` where
  `match_activation.rs:566` passes `Some(ordinals.take(prepared.session()))`, so
  the fixture spells what production spells.

⇒ And the `content_unstated` FALLBACK inside the repaired transition road
(`crates/ambition_platformer2d_runtime/src/room_transition/loading.rs:1228`, reached only when `ActiveContentBinding::live_or` finds no
live binding) is covered by that first arm's positive assertion rather than by
the comment beside it. One arm in nine was on the wrong road; the rest were not.

✔ **AND THE DEFECT SHAPE NOW HAS A RATCHET:
`scripts/one_owner_per_canonical_identity.py`.** It censuses every production
`SimId::<ctor>(literals…)` call — the mints that name ONE specific entity rather
than a family — and reports any constant identity more than one production site
spells. Run against the tree before the repair it reports exactly the defect:
`singleton("session", "root")` at two sites.

⚠ **THE POPULATION IS TWO, AND A RATCHET AT ITS CEILING IS NOT A RATCHET** — so
the arms are the value, not the number. `scripts/tests/test_one_owner_per_canonical_identity.py`
builds its own corpora rather than pinning to a live defect that was repaired the
same day, and three poisons fire three distinct arms: a second production mint of
`session:root` (the repository arm), a per-line scan loop (the wrapped-call arm),
and a literal filter that accepts variables (the `placement(id)` arm). ⛔ A fourth
poison did NOT fire — removing `re.S` changed nothing, because `[^()]` already
spans newlines, so the flag was decoration that read like the fix. It is gone and
the comment says which change the arm actually holds.

⚠ **AND THE INSTRUMENT CANNOT TELL A MINT FROM A REFERENCE.** Its one adjudicated
row, `player_slot(0)`, is one of each: minted in `sim_identity.rs` as the fallback
identity for an unidentified primary-player body, and spelled in `possession.rs`
as the controller a possession claim names. Two sites is right there because the
possession feature is slot-0-only by design and says so at its `home_q` parameter.
A test asserts every adjudicated row still has two sites, so a stale decision
cannot silently excuse a new duplicate.

⭐⭐ **AND THE SECOND MINT IS GONE, WHICH IS THE ACTUAL REPAIR.** A canonical
identity minted in two places is two authorities for one fact whatever both
currently spell, and one of the two was unreachable — so the collapse costs
nothing and removes the thing that made the wrong arm look right.
`ActiveGameplaySession::spawn_world_for` is deleted: it spawned the root itself <!-- cite-ok: the deleted mint is what this row records -->
behind a validation `adopt_world` already performs, making it a duplicate of both
halves at once. What it was propping up, measured by what the compiler said
afterwards:

- `ambition_game_shell` no longer imports `SessionRoot` or `SpawnSessionScopedExt`
  — the shell crate does not spawn session-scoped entities at all now;
- `delayed_world_publication_for_a_cannot_attach_to_b` tested the publication <!-- cite-ok: the row records the name that was deleted -->
  contract on the dead copy and `adopt_world` had NO test; it moved and became
  `a_candidate_world_prepared_for_a_cannot_be_adopted_into_b`, with a positive
  half so an `adopt_world` returning `None` unconditionally cannot satisfy it;
- the ID-PEER provenance arm lost its subject and is retired, with a routing
  comment where it stood naming the road and the arm that hold the claim now.

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

⛔⛤ **THIS BLOCK WAS A FOUR-STEP IMPLEMENTATION ORDER UNTIL 2026-09-16, AND
THREE OF ITS FOUR STEPS HAD LANDED WHILE IT STILL READ AS FUTURE WORK** — the
worst of them saying *"THE PROJECTION IS STILL NOT LANDED"* about a projection
that is registered in production, two screens below a table row recording the
same road CLOSED. Re-derived against source:

1. ✔ **The peer-agreed match ordinal** (schema 187). `ActiveMatch` carries which
   match of its session it is, minted by a rollback-registered
   `SessionMatchOrdinal` that restarts at zero on a session change. It closed
   `random_context` AND `SimId::match_spawn`, which was embedding the absolute
   tick in a `component-canonical` identity string.
2. ✔ **The peer/local split on `CheckpointOperationKey`** (schema 188). The peer
   projection is the admission sequence plus whether a scope owns the operation;
   the scope keeps its local stale-operation job, which was never to be deleted —
   `SessionCheckpointOperations` advances its sequence only on ADMISSION for
   exactly that reason.
3. ✔ **`TransactionId`** — registered
   `rollback_component_canonical_checksum::<TransactionId>` with
   `TransactionId::peer_stable_checksum`, which folds the content identity and
   the room and drops the session stamp and the binding's app-local epoch. The
   type still snapshots WHOLE, because a rewind must restore the local ownership
   A10's candidate-vs-live separation and the construction scope's gather filter
   both read; **only the COMPARISON narrows**, and
   `a_transaction_stamp_depends_on_host_local_lineage_and_must_keep_doing_so`
   holds that half.
4. ⛔ **The timeline itself** — a session-relative tick. Netcode work, and it
   wants `Q128` answered before anyone starts.

⚠ **ONE KNOWN TRADE, RECORDED AT THE CODE AND REPEATED HERE BECAUSE IT IS THE
THING THAT COULD ROT.** `peer_stable_checksum` reads the rendered stamp rather
than structured parts, because a checksum registrar hands a projection only
`&TransactionId` and that is a bare `String` whose `from_raw` is the codec's
decode half. So the formatter in `ConstructionScope::transaction` and the reader
in `peer_binding_term` can drift; what holds them together is the round-trip arm
in that module's tests, which mints a real stamp and projects it. ⇒ Restructuring
the type into its parts with two renderings over one mint — the `MatchInstance`
pattern — remains the cleaner shape and is NOT owed: it buys removal of a drift
risk one arm already covers, at 124 `TransactionId` lines (55 of them under a
test path, `grep -rn "TransactionId" --include=*.rs crates/ game/ examples/`).
⛔ Its precondition is unambiguous parsing, and that is what the three-shape match
on `runtime-dynamic` / `epoch|content:` / bare-epoch buys today.

⚠ **AND THE EPOCH'S OWN MODULE DOC ARGUED THE OPPOSITE UNTIL 2026-09-15** — *"an
epoch is not rollback-registered. Two peers never compare sequences, so a gap on
one host is invisible"*, which was the stated justification for letting a refused
reload BURN a number. Not registered, compared anyway, through whatever embeds
it. Corrected at the definition.

⛔ **THREE PROHIBITIONS THAT MUST SURVIVE ANY FURTHER WORK HERE, each measured
rather than argued.** (1) **The session term STAYS in the string** — the
construction scope's gather filter is what isolates one session's entities from
another's, so removing it breaks A10's candidate-vs-live separation. That is the
split the campaign header warns about: A10 needs exact LOCAL ownership, ID-PEER
needs the token out of the peer COMPARISON. (2) **A projection to `{room}` alone
is worse than the defect** — every entity in a room would share one identity;
this is why the peer-stable CONTENT term had to land first, as
`ambition_platformer2d_core::PeerContentIdentity` beside `ContentEpoch`, with
`canonical_summary` rendering `epoch:N|content:<64 hex>`. ⛔ Why that pair is one
pair and must be minted together is owned by
[`engine/content-generation-and-reload.md`](engine/content-generation-and-reload.md),
not by this row. (3) **The content segment is ABSENT rather than zero-filled when
unstated**, so a binding built outside a prepared session renders `epoch:N`
exactly as before; that is what kept every fixture's identity byte-identical when
production strings moved.

⛔ Do NOT continue by mechanically replacing each raw `SessionScopeId` with the
nearest canonical-looking value. `SimTick` is why: it looks canonical, it
rewinds, it is already checksummed, and it is host-local.

⛔⛤ **A DESIGN CAN ACQUIRE THIS DEFECT BEFORE IT HAS ANY CODE, AND ONE DID ON
2026-09-17.** [Composable actor resources](engine/composable-actor-resources.md)
forbade unordered iteration in layout CONSTRUCTION and said nothing about how the
layout's IDENTITY is derived — so an intern-table ordinal would have satisfied
every rule on the page while giving two peers different ids for byte-identical
layouts, which is `RollbackOrdered` again under a mechanical-sounding name. Its
`R13` now requires a content-derived key or a dense ordinal over a canonically
sorted set inside the admitted generation, and its acceptance arm is this row's
acceptance shape: **the same layout prepared after DIFFERENT irrelevant
histories**. ⇒ When a page names a new id that crosses to a peer, the question is
not whether its construction is ordered; it is what the id is a function OF.

<!-- Stated as a field rather than only in prose: two derivations of "what blocks P0/P1" scanned row prose and missed gates recorded only further down. `scripts/check_blocking_set_names_every_gate.py` reads these lines. -->
**Blocked by:** nothing.

⭐ **RULED 2026-09-19 (Q122):** mechanical identity
fingerprints MECHANICAL FACTS, not explanatory prose. ⇒ The
snapshot-schema-fingerprint road is implementation: exclude comments and other
non-mechanical text from the fingerprint. See
[`maintainer-decisions.md`](maintainer-decisions.md).

⚠ **NOT GATES ON THIS ROW, AND DELIBERATELY BELOW THE FIELD SO A SCAN DOES NOT
READ THEM AS ONE.** `Q128` is the TIMELINE half and is additionally blocked on
`N2`'s absent P2P session; `Q137` gates the twelfth sharp unchecksummed row,
whose other eleven are covered and agreeing.

**Acceptance:** two Apps that have burned different numbers of local session
activations can enter the same deterministic match and produce the same canonical
mechanical identity/checksum. The witness must first assert that their local
counters differ.

### ROLLBACK-KIND-SPELLING — one registration, one kind, spelled once — ✅ DONE 2026-09-16

**Receipt** (landed `5eb6ef3a0`). `ambition_platformer2d_core::rollback_kind::spelling`
holds every (kind, sentence) pair once, both registration roads reference it,
and no literal pair is spelled beside the code. Guard:
`scripts/check_rollback_kind_spelled_once.py`, with
`scripts/tests/test_rollback_kind_spelled_once.py`.

### ROLLBACK-MUTATOR-POPULATION — the mutator guard sees a quarter of rollback state

**Owner:** rollback scheduling (`scripts/check_rollback_mutators_run_in_sim.py`).

**Current state:** the widening LANDED. Population 139 → 338 types, findings
8 → 12, resource half at `068bb6034` and the component half at `fa910c50d`. The
guard exits 0 again as of `c1ffa812d`. What is open is the `Transform` blind
spot; the two findings that owed an argument have it (the ✅ block below).
⚠ What `--list` prints today, 2026-09-18, so the widening's number is not read
off this page a month from now: **342 rollback types** (floor 300), **60 system
param bundles** (floor 55), **521 systems** taking one mutably (floor 470, added
the same day — see the fifth-spelling block below for why that third floor was
missing). The 338 above is the widening's own reference point and is left as
history; the systems count read 518 earlier on 2026-09-18, 516 immediately
before the exclusive-world spelling landed, and 523 before the clone deletion.

⛤ **THAT LAST STEP IS A DECREASE AND ITS CAUSE IS MEASURED, NOT INFERRED.**
523 → 521 is the player-clone relic going: checked out `c1ba8227a` (the commit
before the merge) and `--list` printed 523, then `main` printed 521, so two of
the seven functions in the deleted `app/player_clone.rs` took a rollback type
mutably. <!-- cite-ok: the file is named BECAUSE it was deleted; that is the row's subject --> ⇒ A FALLING population is the one direction this row's floors cannot
catch — the floor is 470 — and it is also the only direction that can be either
progress or a blinded scanner. Re-measuring at both commits is what tells them
apart, and it is cheap enough that a bare *"it went down"* is never the right
note to leave.

⛔ **THE EXIT CODE MEANS "NO NEW OFFENDER", NOT "CLEAN" — read this before
trusting a green.** `ACKNOWLEDGED` is a second table making the OPPOSITE claim to
`WAIVERS`: a waiver says this system's drift across a rewind does not matter and
carries the argument; an acknowledgement says the drift is REAL and names the row
that owes it. **EIGHT** are banked — six here and two to
MENU-RESET-MIDSESSION — and they print to stderr every run. Re-run 2026-09-18
after the ninth was settled, the whole bank, so this number is measured rather
than carried: `adopt_occurrence_checkpoint_from_save`,
`complete_durable_restore`, `compute_music_intent`, `portal_dev_toggle_system`,
`reconcile_roster_with_frozen_topology`, `sync_ldtk_level_set` here;
`grid_menu_action_activated` and `kaleidoscope_menu_action_activated` there —
the guard's own last line, over **517** mutating systems.
✅ **Re-run 2026-09-28: FIVE, all owed here.** The two menu writers left
with MENU-RESET-MIDSESSION (`f86b03189`) and `compute_music_intent` left
earlier; the guard's last line reads 5 acknowledged over **495** mutating
systems. ✅ FOUR the same day: `portal_dev_toggle_system` flipped
`PortalGun.active` on every gun from `Update`; it now writes a
`TogglePortalGunsActive` host intent (`session::host_intents`) and
`portal_guns_active_toggle_system` flips the guns inside the simulation
(`one_press_turns_every_gun_the_other_way`). Left:
`adopt_occurrence_checkpoint_from_save`, `complete_durable_restore`,
`reconcile_roster_with_frozen_topology`, `sync_ldtk_level_set`.
✅ THREE on 2026-09-29: `sync_ldtk_level_set` wrote the active area into
`LdtkRuntimeIndex`, a rewound copy of `RoomSet`'s active room, from `Update`.
The field and the index's revision cursor are deleted, the index is prepared
content with no rollback row (schema 263, waived in `rollback_coverage.rs`
beside `ActiveContentBinding`), and the sync compares the bundles' `LevelSet`
with the active room's levels (`the_bundle_shows_the_active_rooms_levels_and_follows_a_change_or_a_reload`).
Left: `adopt_occurrence_checkpoint_from_save`, `complete_durable_restore`,
`reconcile_roster_with_frozen_topology`.

⛔⛤ **A NINTH ARRIVED BY A WAIVER LOSING ITS ARGUMENT, NOT BY A NEW WRITE, AND
LEFT AGAIN THE SAME DAY BY EARNING A BETTER ONE — 2026-09-18.**
`track_versus_roster`'s waiver said in its own words that it *"rests entirely on
the write preceding the timeline: `maintain_local_session` starts GGRS only once
a live primary player body exists, and at route entry the roster is still
`RosterSeating::Proposed` with no bodies seated."* Read at the source:
`maintain_local_session` (`rollback_ggrs/src/local_session.rs:249`) opens with
`session_world_entity(world).is_some()`, and its three start gates are a SESSION
WORLD, `durable_hydration_is_pending` and `SessionSeatingSource::Pending`.
**There is no body condition anywhere in it**, and measured, the session world
is ALREADY THERE on the frame the arm fires. ⇒ The premise was false, and
DURABLE-HORIZON-CHECKSUM had recorded the same correction for a sibling waiver
two days earlier — *"the body is the later fact, not the shared one."*

⇒ **A WAIVER IS A CLAIM ABOUT THE TREE AND ROTS LIKE ANY OTHER.** This one was
written when it was true of something and never re-read against the system it
names; the guard cannot check a prose premise, so the only defence is re-reading
the cited function when the row is touched. ⭐ What settled it was not the
argument but an instrument: the write is ordered `.before(LocalSessionSet::
Maintain)` and the CHANGE TICKS at frame end say so, which is a fact a guard can
hold. The witness and the tick numbers are in
[MENU-RESET-MIDSESSION](#menu-reset-midsession--the-menu-writes-rollback-state-from-update--closed-2026-09-19);
the entry is back in `WAIVERS` on that argument and the bank is eight.
⭐ **IT WAS TWELVE, AND ALL FOUR DEPARTURES WERE REPAIRS RATHER THAN AMNESTIES.**
The three `persist_*_to_save` mirrors left by being FIXED (into the sim
schedule), and `dispatch_pending_dialog_requests` left at `0f1edee92` by ceasing
to write the save at all — its increment is now
`count_the_dialogue_visit_when_a_conversation_opens`, inside the schedule. ⇒ The
stale check is what made each deletion deliberate rather than convenient: it
reddened the moment a name stopped being a finding and demanded the commit say
which of its two cases applied. ⚠ This paragraph said "nine (seven here)" until
2026-09-18 — one departure the guard had already recorded, and a count in prose
cannot ratchet. ⛔ A banked name the scan STOPS reporting is also
fatal, or the list rots into a second waiver table and absorbs the next system to
take a fixed one's place. Both branches poisoned at `c1ffa812d`.
⇒ Why it exists: at 12 unwaived findings the guard was stuck at `exit 1`, so a
THIRTEENTH could not change its verdict, and `--maintenance` does not run this
script. A check that reports FAILED before and after a regression has stopped
being one. (YardratAmbition's catch.)

⛔⛤ **`Transform` IS A STATED BLIND SPOT AND THE OBVIOUS REPAIR IS MEASURED
DEAD.** 52 of the 64 offenders the component half would have surfaced are
`Transform` writes from camera, sprite and inspection systems; it is excluded BY
NAME with that count beside it, so a green here says nothing about `Transform`.
⇒ The natural fix — classify by a PROPERTY the system states, `Camera`/`Sprite`/
`Text`/`Mesh`/`Light`/`Node`/a projection in the signature — was counted against
those 52: **23 declare such a marker and 29 do not**, and the 29 are presentation
only by NAME (`camera_follow`, `sync_parallax_layers`, `sync_hit_flash_overlays`).
⛔ A system's NAME is not a reading of its write set; that classifier was wrong in
both directions twice on 2026-09-16 alone.
⇒ **So the repair is a DECLARATION, not a cleverer scanner:** presentation
systems that write `Transform` should join a set or mark the entities they move,
turning an undecidable read of source into a fact the code states. That is ~52
systems rather than this script, and it wants a maintainer's view on the shape
before anybody starts — filed as **`Q139`** in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), which puts
the three candidate shapes (a system set, a marker on the moved entities, a
wrapper component the render layer lowers) against where each one puts the cost.

⭐ **EVERY HOLE THIS GUARD HAS HAD FAILED IN THE GREEN DIRECTION, which is why
`POPULATION_FLOOR` is checked BEFORE the findings.** `&mut T` alone saw 1 type of
113; `SessionWorldMut<T>` hid six; `#[derive(SystemParam)]` hid thirteen — that
last one blind to `MovingPlatformSet`, the single type the guard was originally
written around, because a bundle is one identifier in a signature and a struct
field cannot elide the lifetime the pattern required. Each time the report got
SHORTER and CLEANER, which reads as good news.

⛔⛤ **THE FIFTH SPELLING ARRIVED 2026-09-18, AND THE PARAGRAPH ABOVE PREDICTED
ITS SHAPE WITHOUT PREDICTING ITS SITE.** Everything in that list reads a
SIGNATURE. An exclusive-world system's signature is `fn f(world: &mut World)`:
the pattern matches it, extracts the type `World`, finds it unregistered and
moves on — so every write in the BODY, through `world.resource_mut::<T>()`,
`world.get_mut::<T>(entity)` or the session-world helpers, was invisible. **An
exclusive-world system is what a COMMIT EXECUTOR is**, so the writes this hid are
the destructive ones.

MEASURED before the widening: NINE functions reach a registered type that way and
SEVEN were unseen. **`MovingPlatformSet` is among them, for the second time** —
`apply_world_replacement` writes it through an exclusive world. A type that keeps
disappearing is a fact about how many ways a write can be spelled, not about that
type. Population 516 → 523 systems; one new finding,
`commit_confirmed_lifecycle` in `PreUpdate` writing the rollback-registered
`PendingLifecycleCommit`, now WAIVED on a two-part structural argument (the
intent is only taken for a CONFIRMED frame, and the same call installs a newly
built GGRS session, dropping the old ring so no pre-commit frame is restorable).

⭐ **AND THE COUNT THIS ROW KEPT APPEALING TO WAS FLOORED BY NOTHING.** "A fifth
spelling cannot avoid making the count fall" — the count it moves is the MUTATING
SYSTEMS count, which `--list` printed and `POPULATION_FLOOR` did not check. It
does now (floor 470 against 523), so the argument this row has been making in
prose is finally an assertion.

⚠ **BUT THE FLOOR IS NOT WHAT HOLDS THE SPELLING, AND SAYING SO IS THE POINT.**
The widening moved the real tree by SEVEN systems out of 523 — inside any floor
anybody would set. `test_an_exclusive_world_write_is_not_hidden_by_an_empty_signature`
and its poison are what hold it; the floor catches a COLLAPSE. The same reading
that closed the presence-filter guard's `game/*` hole the same day: a floor
defends against a broken instrument, never against one pointed at part of the
tree.

⛔⛤ **ONE WAIVER WENT QUIET BECAUSE A DIFFERENT REPAIR WAS CORRECT, AND IT IS
BANKED RATHER THAN DELETED.** `handle_ldtk_hot_reload` held
`SessionWorldMut<RoomSet>` and `SessionWorldMut<LdtkRuntimeIndex>` on parameters
it only READS — kept mutable so a session-world writer census would not
undercount, which was a program bent to keep an instrument's number up. Demoting
them was right and took the system out of THIS guard's signature-keyed population
in the same stroke. The write it still causes is in a staged closure inside
`reload_ldtk_world_from_disk`, which the new spelling DOES see — but that is a
HELPER, and `collect` attributes a schedule by finding a name inside an
`add_systems` body. ⇒ Recorded in `BLIND_SPOT_NOT_CLEAN_BILL`, because a waiver
deleted for going quiet would cover the next system to take the name.

⇒ **THE NEXT INSTRUMENT IS NAMED AND ITS COST IS MEASURED, WHICH IS WHY IT IS NOT
HERE YET.** One hop of caller attribution — a registered system inherits what the
helpers it calls mutate — would close `handle_ldtk_hot_reload` and the other five
helpers. Measured 2026-09-18 by bare-name matching: **19 pairs, 8 already banked
or waived and 11 FALSE**, because the helper side collapses to three names —
`tick`, `apply`, `install` — and a `\bname\s*\(` search cannot tell
`adopt_the_ledger(world)` from `self.timer.tick(dt)`. ⛔ A NAME IS NOT A CALL,
and TEN of the eleven are `&mut self` methods on a timer, an animator, a grid,
a shop transaction, a load coordinator or a router — `playback.tick(..)`,
`coordinator.apply(..)`. ⭐ THE ELEVENTH FAILS DIFFERENTLY AND IS WORTH STATING
PRECISELY, because "method collision" is not what happened to it.
`spawn_dialogue_runner <- install [AuthoredOccurrences, CustodyBaseline,
MintedItemBaseline, OccurrenceBaseline]` matches
`crates/ambition_dialog/src/bridge.rs:87`, where `install` is a LOOP VARIABLE —
`for install in &content_bindings.installers` — bound to a
`YarnBindingInstaller = fn(&mut Commands, &mut DialogueRunner, &YarnStateMirror)`
(`crates/ambition_dialog/src/bindings.rs:80`). So it IS a genuine call to a genuine function, of a
same-named family whose three-argument signature cannot be the one-argument
`install(world)` that mutates those four baselines — and none of
`ambition_dialog`, `ambition_conversation` or `game/ambition_content` names any
of the four at all. ⚠ A signature-incompatible namesake and a method call are
two different ways a bare name lies, and a hop that fixed only one of them would
still import this pair. (Independently re-read by CalculexAmbition, who found
the mechanism this paragraph originally got wrong.) ⇒ The hop
wants real call resolution; a version built on the bare name would import
eleven fabricated findings on its first run.

**✅ Two findings owed their own argument, and it is now MEASURED.**
`adopt_occurrence_checkpoint_from_save` and `complete_durable_restore` are
one-shot latches on `SaveRestored`, which LOOKS like the activation waiver's "the
write precedes the timeline". ⛔ **The opposite is what happens.** The GGRS session
is live before the latch flips, and a sampler with an explicit
`.after(complete_durable_restore)` edge finds it already live at the instant the
latch is set. ⇒ They are not even gated on the same fact: GGRS starts on
`session_world_entity(world).is_some()`, the restore chain waits for a primary
player BODY, and the body is the later fact. The measurement, the repeat counts
and the observer effect that moved the start by a frame are in
[DURABLE-HORIZON-CHECKSUM](#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update).
⚠ So neither may be waived on the activation argument, and both stay
ACKNOWLEDGED rather than moving to `WAIVERS`.

✔ **AND THE DISCRIMINATOR IS NOT "IS THERE A REBASE" — IT IS "IS THE TRIGGERING
MESSAGE CLEARED ON ROLLBACK", WHICH WAS ASKED OF `SessionScopeActivated` ON
2026-09-16 AND SEPARATES THE TWO.** (The New Game side is deleted: since
2026-09-29 a New Game is a checkpoint restore and has no commit message.) A New
Game's `NewGameResetCommitted` was
produced inside the rewind window, is `message-clear` in the schema
(`message.sandbox_reset_committed`, baseline line 378) and is consumed in
`Update` — broken whether or not the room replacement rebases GGRS.
`SessionScopeActivated` is a different shape at every end, measured:

| | `NewGameResetCommitted` | `SessionScopeActivated` |
|---|---|---|
| written by | inside the rewind window | `translate_shell_session_lifecycle`, literal `Update` (`crates/ambition_game_shell/src/session.rs:394`) |
| read by | `Update` | `reset_session_scoped_resources_on_activation`, literal `Update` (`crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs:530`) |
| rollback schema | `message-clear` | **absent — no registration at all** |

⇒ Both ends are outside the rewind window and the buffer never enters it, so
there is nothing for a clear to drop. `SessionScopeActivated` owes nothing here.
⚠ Its real exposure is the other one its two waivers already name: they rest on
that sole writer staying in literal `Update`, **re-verified 2026-09-16** — one
production `.write(SessionScopeActivated(..))` site, registered in `Update` — and
they go stale together with `ActiveSessionScope`'s and `SessionScopeRetired`'s the
day it moves into `app.sim_schedule()`.

**Receipts, one line each, so the next reader does not re-derive them.**
⛔ The costed `install<T>(owner, name, kind, detail, ops)` redesign this row
carried DOES NOT TYPECHECK — the methods' `T` bounds are disjoint, so `install`'s
bound list must be their union and every default body fails `E0277`; a 30-line
probe refuted it more cheaply than the paragraph that proposed it.
⛔ A run condition is NOT a reachability proof: 7 menu systems were recorded REAL
because they carry `.run_if(simulation_authorized)`, and five cannot reach the
write at all — the bundle over-grants. Two are real and are MENU-RESET-MIDSESSION.
⛔ Demoting the 21 `derived`-documented clone registrations is NOT part of this
and should not happen on a keyword match: most say "derived" about something else
(`ActorRenderSize`'s collision box, `CapturedBy`'s inverse), and a component whose
PRESENCE a query filter reads is authoritative even when its value is derived.
⚠ `BLIND_SPOT_NOT_CLEAN_BILL` is empty because it was CURED, not tidied; the
mechanism stays, since an empty dict still reddens on a newly stale waiver.
⭐ The widened guard independently names both defects found by harness tonight —
the three `AmbitionGameSave` mirrors ([Q129](awaiting-maintainer-decision.md#q129--must-the-save-file-be-part-of-what-two-peers-agree-on))
and the two menu writers ([MENU-RESET-MIDSESSION](#menu-reset-midsession--the-menu-writes-rollback-state-from-update--closed-2026-09-19)).
Neither needed a new idea, only a population nobody had quietly narrowed.

<!-- Stated as a field rather than only in prose: two derivations of "what blocks P0/P1" scanned row prose and missed gates recorded only further down. `scripts/check_blocking_set_names_every_gate.py` reads these lines. -->
**Blocked by:** nothing.

⛔ **RULED 2026-09-19 (Q139):** do NOT grow
architecture merely to satisfy a static presentation-writer census. ⇒ None of
the four declaration shapes this row costed is to be built for the census's
sake; the `Transform` blind spot is accepted as a known limit of the scanner,
and any future repair must be justified by a real mechanical failure rather
than by the guard's coverage. This ruling arrives with the same day's priority
adjustment, which says the same thing in general terms. See
[`maintainer-decisions.md`](maintainer-decisions.md).

**Acceptance:** the population is every rollback registration, not one
registration spelling; `handle_ldtk_hot_reload` is visible without its waiver
being deleted; a poison that respells a write in any supported param form still
reddens the guard; and the population floor fails when a spelling stops matching.

### DUP-SESSION-CURRENT — one owner per session-identity question — ✅ DONE 2026-09-16, RE-MEASURED 2026-09-18

**Receipt.** `GameplaySessionInstance` in `ActiveGameplaySession` is the one
owner of the live session's (activation, scope) pair. The one-entry copy of that
pair is deleted (`2ca6469a3`); `GameplaySessionWorldRoot` and
`GameplayInputOwner` stay as correlations on the entities they describe.
`ReservedGameplayScopes` is not a second owner, because a reservation exists
only before its activation. Witnesses:
`a_retirement_that_arrives_after_its_session_ended_changes_nothing` and
`a_candidate_session_the_transaction_refuses_leaves_the_live_session_playable`.
The ledger row is in
[`consolidation/architecture-census.md`](consolidation/architecture-census.md).

### SETTINGS-ROLLBACK — finish the settings/mechanics admission boundary

**Owner:** rollback/mechanical-policy owners.

**Current state:** direct `UserSettings` reads have been removed from the
simulation schedule — `scripts/measure_user_settings_in_simulation.py` reports
ZERO simulation readers. That removed the 30-field menu-mutable resource from the
sim and cut four readers to one writer; it did NOT make either policy
deterministic.

⛔⛤ **BOTH HALVES WERE FORWARD-ONLY, AND THIS ROW SAID SO ABOUT ONLY ONE OF
THEM.** Corrected 2026-09-16. It read *"Control-frame modes are projected per
seat, and damage uses `PlayerDamagePolicy`. The remaining damage policy is still
forward-only"* — which put the frame-mode half in the done clause and left the
damage half as the remainder. Measured, both were the same shape; one has since
closed:

| policy | writer | schedule | rollback-registered? | sim readers |
|---|---|---|---|---|
| `PlayerDamagePolicy` | `project_player_damage_policy` | **`Update`** | **no** (0 rows in `rollback_schema_baseline.txt`) | 3 — still open |
| `SeatControlFrameModes` | `populate_seat_control_frames` | `Update` | no (0 rows) | **0 — CLOSED 2026-09-16**, the policy rides `ControlFrame` |

⇒ A resimulation of frame N read whatever either policy held NOW. That is still
true of the damage half.

⭐ **AND THE ROAD THAT MOVES IT MID-SESSION IS NAMED NOW — MEASURED 2026-09-19,
because "forward-only" is only a defect if something can change the value while
a timeline is live.** It can. `project_player_damage_policy` reads
`UserSettings.gameplay`, and `Difficulty`, `Assist` and `PlayerDamage` are
ordinary rows of the settings model
(`crates/ambition_settings_menu/src/settings/apply.rs`), which the in-game
System face surfaces generically as `SystemRow::Setting(..)`
(`game/ambition_app/src/menu/model.rs:493`). That overlay opens from
`GameMode::Playing` and restores it on close. ⚠ **THE PAUSE MENU IS NOT THE
ROAD**, and checking it first was the wrong place to look: `PauseEntry` is
`Resume`, `Abandon`, `Audio`, `QuitToTitle`, `QuitToDesktop`, `Close` — no
settings row at all.
`rollback_coverage.rs` waives each one with that stated in its reason; this row is
where a reader looks first, and it was the copy that had drifted. ⚠ "Projected" is
not "admitted": a projection narrows who reads a mutable value, and the timeline
question is untouched by it.

**Blocked by:** nothing.

⛔ **RULED 2026-09-19 (Q127), AND THE ANSWER IS
"DEPRIORITISE THIS":** there is to be NO generic one-dimensional engine
difficulty architecture. Difficulty is game policy expressed as presets;
participant handicaps and CPU brain levels are SEPARATE concepts from match
policy, and participant-specific assist/handicap state stays distinct from
game/match policy. ⇒ The damage half is not blocked — it is deferred on
purpose. Preserve enough architecture not to be boxed in later and spend no
substantial effort here until the default/Normal game plays exceptionally well.
See [`maintainer-decisions.md`](maintainer-decisions.md).

⭐⭐ **AND THE DETERMINISM HALF HAS A SHIPPED PRECEDENT THAT NEEDED NO PRODUCT
RULING — FOUND 2026-09-19 WHILE COUNTING THE EDITOR DOMAINS.** `PortalTuning`
was in exactly this state: the settings menu wrote the authority directly,
`.before(portal_transit)`, which under the rollback host is inside
`GgrsSchedule`, so *"a replay of frame N observed whatever the settings menu
holds NOW, and it overwrote whatever the admitted portal-editor publisher had
just published"* (`game/ambition_content/src/portal/plugin.rs:218`). The repair
asked for no NEW ruling — it applied one that already existed: `sync_portal_reorient_from_settings` authors the MIRROR and
PROPOSES, in `MechanicalEditSet::Propose`, so the settings menu and the F-key
panel write the same field in the same place and
`publish_editable_portal_tuning` stays the only writer of the authority. That
is `Q120`'s admission protocol, which is now a recorded ruling.

⇒ **SO `Q127` DECIDES THE KEY, NOT THE SHAPE.** Whether the damage modifiers
are one match-wide value or per participant changes the TYPE the protocol
carries; that the settings road must propose rather than write is already
settled everywhere else this pattern occurs. ⚠ The rework risk is real and is
why this is recorded rather than done: a per-participant answer rewrites the
resource the wrapper wraps. What the row should not do is keep reading as if
the determinism question itself were waiting on a product call.

⚠ **MEASURED WHILE HERE, because `status.md` states this count and a count
crossing a document boundary carries its method:** SIX editor domains —
movement tuning, abilities, developer body profile, player stats, feel tuning,
portal tuning — registered at three sites, with SEVEN proposers (portal has
two: its F-key panel and the settings mirror) and six publishers. ⇒
`status.md`'s *"six current editor domains"* is right. ⚠ The app canary says
*"five proposers and five publishers"* for `build_visible_app`
(`game/ambition_app/tests/developer_edits_under_rollback.rs:330`) — a
COMPOSITION count beside this WORKSPACE count, not reconciled here. Whoever
needs the difference should measure the composition rather than subtract.
⭐ **THE FRAME-MODE HALF WAS NOT BLOCKED ON A RULING AND HAD A RECORDED REPAIR**
(kept below because the repair it named was NOT the one taken, and the reasons
are the decision record).
The architecture review of 2026-09-13, as `rollback_coverage.rs` records it:
*"The 2026-09-13 review said capture should resolve the semantic DIRECTION so
simulation sees no mode."* ⛔⛤ **THAT WAS PRINTED HERE AS A VERBATIM QUOTATION
UNTIL 2026-09-19, WITH A TRAILING CLAUSE — *"at which point this waiver and the
row above both shrink"* — THAT APPEARS IN NO FILE IN THIS REPOSITORY**, source
or planning. The file states the claim in INDIRECT speech and the claim is
right; the quotation marks were the part nothing could check. That is
implementation work,
not a decision — and it is strictly better than admitting the mode as state,
because it removes the concept from the simulation rather than versioning it.

⛔⛤ **AND THE RECORDED REPAIR HAS A COST NOBODY WROTE DOWN, MEASURED 2026-09-16
BEFORE STARTING IT.** Resolving the direction at capture so simulation sees no
mode is implementable, and it is not free, because resolving
a direction needs the controlled body's gravity BASIS and capture does not have
frame N's:

- `AccelerationFrame::resolve_input` (`ambition_geometry/src/reference_frame.rs`)
  needs the basis for two of its three modes — `ScreenRelative` computes
  `input.dot(self.side)` and `input.dot(self.down)`, `BodyRelativeAssist` reads
  `self.down.y`. Only `BodyRelativeStrict` ignores it.
- The basis comes from `AccelerationFrame::new(gravity_dir)` where `gravity_dir`
  is `controlled_frame_down(...)` reading `ResolvedMotionFrame`, which the
  baseline lists as `derived` — *"published every tick from the live
  environment"*.
- `populate_seat_control_frames` runs in `Update`, ONCE per real frame, while the
  sim may advance and resimulate many frames inside that one update.

⇒ So a capture-resolved direction is baked against whatever basis the LATEST
completed tick left behind, not frame N's. That is still deterministic and still
peer-correct — GGRS replays the resolved value — but it changes the mechanic:
**under a gravity flip, a gesture resolves in the basis that was current at
capture rather than at its own frame.** Gravity does flip mid-match on the
production road (`FlipGravity` is an authored `Switch` action handled in
`drive_wave_encounters`; the dead overlap plate was deleted 2026-09-19).

⭐ **THE ALTERNATIVE IS EQUALLY DETERMINISTIC AND KEEPS THE BASIS LIVE: CARRY THE
MODE IN THE INPUT.** `AmbitionGgrsConfig = GgrsConfig<ControlFrame>`, so a field
on `ControlFrame` travels with the input and is replayed per frame; the sim then
applies the mode against the basis it already holds for frame N. This is the shape
`SeatControlFrameModes`' own doc names — *"a remote seat's row is filled from
whatever travels with that peer's input, and no simulation call site moves"* — and
`ControlFrame`'s doc says the cost is low: *"adding a `ControlFrame` field does not
bump `INPUT_STREAM_VERSION`"*, the struct is `#[serde(default)]`, and no `Pod`
bound applies.

⇒ **BOTH SHAPES FIX DETERMINISM AND BOTH FIX THE PEER HALF. They differ only in
which basis a gesture resolves against under changing gravity, which is a feel
question and not a netcode one.** That is the choice this row now records; it was
not visible when the repair was written down.

⛔⛤ **AND "THEY DIFFER ONLY IN FEEL" IS TOO KIND TO SHAPE 1 — THE DEVIATION LANDS
ON THE DEFAULT CONFIGURATION, NOT AN OPT-IN ONE.** Measured 2026-09-16 from the
definitions rather than from the names:

| | value | where |
|---|---|---|
| default camera frame | `WorldFixed` (`#[default]`) | `CameraReferenceFrame` |
| default movement mode | `ScreenRelative` | `InputFrameMode::DEFAULT_MOVEMENT` |
| default aim mode | `ScreenRelative` | `InputFrameMode::DEFAULT_AIM` |

`InputFrameMode::under_camera` collapses every mode to `BodyRelativeStrict` under
a `SubjectFrame` camera — which would have made this whole question moot — but
that is not the default, so nothing collapses on the shipped road.

And `resolve_input`'s BODY, not its doc: `ScreenRelative` is
`input.dot(self.side)` / `input.dot(self.down)`, and `BodyRelativeAssist` is
`if self.down.y < 0.0 { -1.0 } else { 1.0 }`. Two of the three modes are DEFINED
as a function of the CURRENT basis; only `BodyRelativeStrict` ignores it, and it
is the default for nobody.

⇒ So shape 1 resolves the default player's every gesture against a stale basis
for the capture-to-frame lag, at each gravity flip — and `ScreenRelative`'s own
contract is *"the body moves the way the stick points ON SCREEN at any gravity"*.
That is not a preference being retuned; it is the default mode not doing the one
thing that distinguishes it from `BodyRelativeStrict`, transiently.

⚠ **WHAT IS NOT MEASURED, AND THE ROW SHOULD NOT PRETEND OTHERWISE:** the SIZE of
that lag in frames (one `Update`'s worth of sim frames, more under catch-up), and
whether it is perceptible at the rate authored gravity flips actually turn. Shape
1 could still be chosen deliberately, trading a bounded transient for removing
the concept from the simulation. What the row can no longer say is that the two
are symmetric: shape 2 has no such transient, and `ControlFrame`'s own doc prices
its cost at no `INPUT_STREAM_VERSION` bump, `#[serde(default)]`, no `Pod` bound.
⇒ **The evidence points at shape 2; a maintainer choosing shape 1 is accepting a
named cost rather than picking between equals.**

⛔⛤ **AND SHAPE 2'S PRICE IS QUOTED FROM THE WRONG LEDGER.** This row prices it
as *"adding a `ControlFrame` field does not bump `INPUT_STREAM_VERSION`"*, and
that sentence is TRUE — verified at both ends, `ControlFrame` really is
`#[serde(default)]` with no `Pod` bound, and `INPUT_STREAM_VERSION`'s own doc
says *"ADDING a field does not need a bump… an older stream loads with the new
field neutral, which is exactly what an older recording meant by it."*

But `INPUT_STREAM_VERSION` versions RECORDED REPLAY FILES. It is used in exactly
three files, none of them a handshake, and the question it is being used to
answer here is a PEER one. Measured 2026-09-16, every candidate that could cover
the peer side:

| candidate | covers `ControlFrame`'s shape? |
|---|---|
| `INPUT_STREAM_VERSION` | no — replay streams, and it exempts added fields by design |
| rollback schema dump | no — one row, `derived.control_frame`, carrying the type NAME only |
| `schema_fingerprint` | no — it hashes that dump, so it sees the name, not the fields |
| `scripts/tests/rollback_codec_shape.txt` | no — zero mentions; `ControlFrame` has no `SnapshotState` impl, because it is `derived` and rebuilt from the input stream rather than snapshotted |

⇒ **NOTHING IN THE REPOSITORY VERSIONS THE SHAPE OF THE PAYLOAD TWO PEERS
EXCHANGE.** `AmbitionGgrsConfig = GgrsConfig<ControlFrame>` makes `ControlFrame`
the GGRS input type, so it is literally what crosses; the state schema now has an
identity AND a ratchet, and the input half has neither. Filed as ID-PEER's
fifteenth road. It does not block shape 2 — shape 2 is what EXPOSES it, and
adding the field is no worse than the exposure that already exists — but the row
may not go on quoting a replay-file version as though it priced the peer cost.

✔ **THE FRAME-MODE HALF IS CLOSED (2026-09-16), BY SHAPE 2.** Four
`Res<SeatControlFrameModes>` parameters are gone from the simulation schedule;
`ControlFrame` carries `control_frame_modes`, stamped at capture in BOTH branches
of `populate_seat_control_frames` (a paused seat keeps its preference, for the
reason the table was already published before that early-out), and GGRS replays
it per frame — so a resimulation of frame N reads the mode frame N was CAPTURED
with.

⭐ **SHAPE 2 WAS CHOSEN BECAUSE IT IS BEHAVIOUR-PRESERVING, WHICH ALSO MEANS IT
DOES NOT FORECLOSE SHAPE 1.** Carrying the mode leaves the gravity basis live, so
every gesture resolves exactly as it did before; only the mode's TIMELINE
changed. Anyone who later prefers capture-resolved directions can still take that
road, and would be accepting the named cost above rather than undoing this.

⇒ **WHAT IT COST, measured:** `CONTROL_FRAME_WIRE_IDENTITY` 1 → 2 with its
baseline re-frozen (48 rows), and **the rollback schema did not move at all** —
the dump records `derived.control_frame`'s TYPE, not its fields, so no
`GGRS_ROLLBACK_SCHEMA_VERSION` bump was owed. That is the fifteenth road's gap
seen from the inside: the state identity was blind to this change and the input
identity, built hours earlier, caught it.

⭐ **AND THE NEW RATCHET CAUGHT ITS OWN AUTHOR.** Adding the field raised
`input_payload_shape`'s transitive refusal — `ControlFrameModes` is a second
non-primitive field type whose shape moves independently — so the census now
follows it and `InputFrameMode`'s variants too. The boundary was asserted rather
than assumed complete, and it fired on the first change that crossed it.

⚠ **TWO THINGS THE CLOSURE DID NOT MAKE DISAPPEAR.** The waiver in
`rollback_coverage.rs` SURVIVES, because that guard's population is every mutable
Ambition resource and not the types simulation reads — measured by deleting it
and watching two arms redden. Its REASON was rewritten to the new truth rather
than left describing a repair that has happened. And
`measure_user_settings_in_simulation.py`'s control for this projection DIED of
success: "no reader at all" meant instrument failure, which was right until zero
became the correct answer. An absence now needs a presence premise — the type's
own declaration — so closed, renamed and never-looked print differently.

**Next implementation:** the damage half only.
1. ~~Frame modes~~ — CLOSED above; the acceptance measurement is
   `scripts/measure_user_settings_in_simulation.py` reporting **0 simulation
   readers of `UserSettings` and 3 of its projections**, down from 3 + 4.
2. **Damage (after Q127):** make the admitted policy follow the chosen lifetime —
   match activation if match-wide, deterministic per-seat input if
   participant-specific. Do not reintroduce simulation reads of mutable
   `UserSettings`.

**Acceptance:** rewinding/resimulating frame N observes the policy admitted for
that timeline, not whatever the settings UI contains now; the settings-to-policy
projection remains witnessed end to end; and **no `sim`-schedule system takes
either policy resource as a parameter**, which is the check that distinguishes a
projection from an admission and would have caught this row's drift.

⛔ **AND THE PEER HALF — WHOSE PRINCIPLE WAS ALREADY RECORDED AND WHOSE CALL SITE
WAS NOT.** ⚠ The principle is not new and I nearly published it as if it were:
`scripts/measure_user_settings_in_simulation.py` prints it in its own
classification — *"resolve at the INPUT-CAPTURE boundary so deterministic
simulation consumes semantic intent; **peers must not have to share accessibility
settings**"*. What was not written anywhere is the measured consequence, which
needs no settings change to fire: `holding_descend(control.axis_x, control.axis_y,
gravity_dir, movement_mode)` in `possession.rs` takes the AXES from GGRS's
replayed input and the MODE from an App-local preference, so two peers holding
different preferences interpret the same exchanged input differently from the
first frame, and in that call site the mode decides which way "down" is for a
control-authority transfer. ⇒ So the frame-mode repair above is not only a
determinism fix; it is the only one of the two shapes that is peer-correct,
because a direction resolved at capture travels with the input while a policy
resource does not.

ⓘ **RE-MEASURED 2026-09-16 RATHER THAN QUOTED:**
`scripts/measure_user_settings_in_simulation.py` at this HEAD reports **44
production functions taking `Res<UserSettings>` and 0 of them inside the
simulation schedule.** The three damage readers it and the waiver name —
`apply_player_hit_events`, `apply_feature_hit_events`, `charge_projectile_input` —
are each registered in `player_schedule.rs` or `combat_schedule.rs`.

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

⛔ **MEASURED 2026-09-16 AND BOTH HALVES ARE BLOCKED, FOR TWO DIFFERENT REASONS.**
The row was implemented in full and reverted. The acceptance above is MET at the
unit level and that turned out not to be worth much — see the staleness half.

**THE RAGE HALF IS A RETUNE, so the sentence "not a request to retune all
throws" is false as measured.** `apply_capture_throws` multiplying its launch by
`rage_scale(thrower_damage_taken)` — the shipped cap is `1.4`, and the largest
multiplier anywhere in the duel is `1.17` — costs the CPU duel this, sweeping
`AMBITION_DUEL_RUNG` over all five published rungs:

| rung | HEAD | with rage on throws |
|------|------|---------------------|
| 1 | 1.99 ✅ | 2.07 ✅ |
| 3 | 1.99 ✅ | 2.29 ✅ |
| 5 | 0.21 ❌ | 0.21 ❌ (bit-identical) |
| 6 | 2.32 ✅ | 0.64 ❌ |
| 9 | 1.36 ✅ | 0.46 ❌ |

Median `1.99 → 0.64`; three of five rungs fail
`two_cpus_in_the_shipped_composition_damage_each_other` where one did. The
failing duels are LONG and UNDECIDED (3618 ticks, `decided None`) — harder
throws separate the fighters instead of killing them.

⚠ **AND IT IS NOT CHAOTIC RE-ROLL, WHICH IS THE FIRST THING IT LOOKS LIKE.** A
flat `×1.05` on the throw with no rage and no staling reproduces the rung-9
failure to the digit (3618 ticks, 18/43 damage, `0.23 / 0.23`). But the metric is
not monotonic in that constant — `×1.01 → 1.36`, `×1.02 → 1.18`, `×1.04 → 1.03`,
`×1.05 → 0.46` — so a single rung cannot tell a retune from a coin flip, and the
five-rung sweep is what can. Rung 5 coming back BIT-IDENTICAL under the change is
the instrument's own control: no throw lands there, so the road provably did
nothing, exactly where it should do nothing.

⇒ [RULED](maintainer-decisions.md) 2026-09-19 (`Q133`): for the Smash-like
game, follow Smash — ordinary scaling throws participate in rage, and
set-knockback keeps its set-knockback semantics as in Ultimate. ⛔ Not a
universal engine law: rage, and whether a given move or throw obeys it, are
game-level combat policy the engine must be able to EXPRESS. ⚠ If the CPU-duel
benchmark moves when throws obey rage, that is combat/AI/balance evidence, not
a reason to keep a mechanics inconsistency.

⛔⛤ **THE STALENESS HALF IS A READ WITH NO MATCHING WRITE — it would be inert
forever and its witness would still be green.** Wear is recorded at exactly ONE
site, `moveset/mod.rs::mark_move_playback_landed_hits`, gated on
`hitbox::LandedBodyHit`; that message is written at exactly one site,
`hitbox/mod.rs:1128`. `apply_capture_throws` does not mention `LandedBodyHit` at
all — it applies `health.damage(request.damage)` directly. So a throw never
records its own use, and `occurrences` for a THROW-ONLY move id (`pirate_fthrow`)
is structurally always `0`. Measured: staleness routed into the throw with rage
held neutral is bit-identical to HEAD on rungs 1, 3, 6 and 9.

⚠ **THE UNIT WITNESS THE ACCEPTANCE ASKS FOR PASSES ANYWAY, WHICH IS THE WHOLE
TRAP.** A controlled throw witness seeds `BodyStaleMoves` by calling
`queue.record(..)` itself, so it proves the arithmetic and says nothing about
whether the game can ever reach a nonzero `occurrences`. The one written here was
green while the shipped composition got nothing. ⇒ Staling throws needs a SECOND
edit this row does not mention — recording the throw's use — and that is a
mechanics question (does a throw stale the throw, or the grab?) that belongs in
the row before any code does.

⇒ ⛔ **Do not re-implement either half from the row text alone.** It reads as one
20-line change and it is not.

⭐⭐ **BOTH CLAUSES ARE LANDED — set knockback in `45b30500e`, throws-obey-rage
beside this line.** The ruling has two clauses and they separate cleanly in the
tree, so they separated in the commits and the second one's cost was measured
against the first as a control.

**Landed.** Rage now resolves inside `resolved_hitbox_knockback_magnitude`
instead of at its caller, and a launch whose AUTHORED growth is zero declines
it. The caller could not make that call: the two authoring roads for growth (an
explicit `Some(g)`, the ruleset's `base * ruleset_growth`) are collapsed inside
the resolver, so outside it a set launch and a weak one look the same. The
predicate reads the collapsed AUTHORED growth, before `growth_base` and
`growth_scale`, because both are ruleset knobs that can reach zero and would
switch rage off game-wide if they were allowed to answer this.
⭐ MEASURED: bit-identical to HEAD on the duel at rungs 6 and 9 — no shipped
set-knockback volume lands in that bout, so the change is free where it was
measured and correct where it is not.

**Landed, and its cost was evaluated rather than escalated.**
`crate::util::rage_for_growth` was shared so the throw road could call it;
the throw side is four lines (`captors` gains `Option<&BodyHealth>`,
`apply_capture_throws` keeps the whole `ResolvedCombatTuning` instead of
projecting one field out of it, and the resolved magnitude takes the captor's
resolved rage). It is covered by
`a_hurt_captor_throws_farther_and_a_set_throw_is_immune`, whose two arms were
poisoned separately and reddened through their own assertions.
⇒ **UPDATED 2026-09-20:** `rage_for_growth` no longer exists as a function. The
set-launch-declines-rage rule is a branch INSIDE
`ambition_entity_catalog::launch::launch_speed`, which both roads now call —
see the launch-law block on the [BRAIN](#brain--finish-truthful-fighter-attack-selection)
row. The rule is unchanged; it has one owner instead of two callers.

⛔⛤ **AND IT HALVES THE SHIPPED CPU DUEL, WHICH REPRODUCES 2026-09-16 EXACTLY.**
Re-measured 2026-09-19 against today's tree, `AMBITION_DUEL_RUNG` over all five
published rungs, `npc_pirate_admiral` mirror:

| rung | HEAD today | set-knockback half only | + throws rage |
|---|---|---|---|
| 1 | 1.99 ✅ | — | ✅ |
| 3 | 1.99 ✅ | — | ✅ |
| 5 | 0.21 ❌ | — | 0.20 ❌ |
| 6 | 2.32 ✅ | **2.32 ✅ bit-identical** | 0.64 ❌ |
| 9 | 1.36 ✅ | **1.36 ✅ bit-identical** | 0.46 ❌ |

Rung 5 fails at HEAD and is not this row's. The whole regression is the throw
clause, and the bit-identical column is what proves it.

⭐ **THE MECHANISM, WHICH THE 2026-09-16 ROW DID NOT HAVE.** Rung 9 with throws
raging runs the full 3618-tick budget `decided None`, where HEAD decides at
2314. The seats are together for 386 of 3618 ticks (11%) against HEAD's 518 of
2314 (22%), and the move census says what they do instead: `grapeshot` starts
go 16 → 56 and `call_the_shark` — the up-B recovery — goes 15 → 38 across the
two seats. Damage dealt INTO THE OTHER SEAT falls 51 → 20. ⇒ A harder throw
sends both fighters off-stage more often; they spend the bout recovering and
trading projectiles, and the duel's damage RATE collapses while its length
grows.

⛔ **AND THE RECOVERY'S LENGTH IS NOT THE LEVER.** `SHARK_RIDE_SECONDS` 5.0 →
2.0, measured with throws raging, is **bit-identical at rung 9** (0.23/0.23,
3618 ticks, 386 ticks within 60px, same move census to the count). Do not spend
the next attempt there.

⚠ **THE GATE'S HEADROOM AT ITS DEFAULT RUNG IS ~4% OF THROW STRENGTH.** The
2026-09-16 flat-multiplier sweep is the calibration: `×1.01 → 1.36`,
`×1.02 → 1.18`, `×1.04 → 1.03`, `×1.05 → 0.46`, against a floor of `1.0`. The
shipped rage is `rage_per_damage: 0.004` capped at `1.4`, which is at most
`×1.17` and averages well under that over a bout. So the reading is real and the
instrument is steep, both.

⇒ **THE GATE WAS RECALIBRATED AND THE MECHANIC LANDED, WHICH IS THE ORDER JON
ASKED FOR.** 2026-09-19: *"You are mistaking these balance sheets as hard
rules. They are references for when we work on balance. They shouldn't prevent
landing features. But they should make us aware when feel changes… We look at
sizes of regressions and then evaluate them… We need to be able to make these
semantic judgements without my input."*

The judgement, stated so the next reader can disagree with it: `1.36 → 0.46` is
a TAPER, not a collapse. The bout still spends three stocks, both seats still
enter hitstun (32 ticks each), and both still deal damage into each other; what
changed is that it takes 3618 ticks instead of 2314. A collapse would read near
zero with a passenger seat, and the per-seat arms that refuse exactly that did
not move. ⇒ `A_REAL_FIGHT` drops `0.5 → 0.125` — a pair floor of `0.25`, which
the two live rungs clear by 84% and 156% and an inert pair still fails — and
the constant's doc now says it answers *"did a fight happen"* and nothing
finer. The graded reading it used to carry is PRINTED on success in the
`[duel]` line, which is where a balance reference belongs.

⚠ **WHAT IS STILL OWED IS TUNING, AND IT IS NOT THIS ROW'S.** Rung 5 reads
`0.20` and failed at HEAD too, so it is a pre-existing defect this change did
not cause and did not fix. And the mechanism above names the real target for
whoever picks up feel: a launched fighter spends 74% of the bout airborne and
fills it with projectiles.

### DUEL-GUARD-RUNG — the CPU duel guard fails at rung 5 on main today

**Owner:** the diagnostic half is DONE; the remaining acceptance belongs to
[BRAIN](#brain--finish-truthful-fighter-attack-selection). Found 2026-09-16
while measuring THROW-MODIFIERS; unrelated to it.

⛤ **THIS SAID "unowned" UNTIL 2026-09-19 AND BOTH HALVES HAD MOVED.** Step 1
landed, and step 2's instruction — *"Re-file the failure there"* — was carried
out: `BRAIN` now holds the rung-5 measurement with its own grid (11/7 distinct
moves, 70%/84% on two moves, 16 of 105 damage reaching an opponent). ⇒ Nothing
here is unowned work waiting for somebody to notice it. **This row's acceptance
cannot be met by anything done in this row**, because it requires rung 5's CPUs
to fight each other, and that is a brain policy defect. Leaving it marked
unowned invites a second person to re-measure a defect that is already filed,
diagnosed and owned — which is how the first version of this row went wrong in
the other direction.

**Current state:** `two_cpus_in_the_shipped_composition_damage_each_other` runs
at `RUNG_DEFAULT = 9` and passes. Its own doc says *"Sweeping the lower rungs is
how that claim is checked against the composed app"*. Swept, at HEAD, with
nothing modified: **rung 5 FAILS**, `0.13 + 0.08 = 0.21` against a floor of
`1.0`. Rungs 1, 3, 6 and 9 pass (1.99, 1.99, 2.32, 1.36).

⛔⛤ **THE GUARD IS RIGHT AND THE DIAGNOSTIC BESIDE IT IS WRONG — CORRECTED
2026-09-16, SAME DAY, AND THE FIRST VERSION OF THIS ROW IS THE ERROR WORTH
KEEPING.** It read rung 5's printed *"105 damage across six moves, 4 knockouts,
MORE than rung 9's passing 95"* and concluded the metric was pool-normalised and
not comparable across rungs, estimating rung 5's pool at ~6× rung 9's by dividing
damage by percent. ⇒ **That is refuted at the source:**
`smash_roster_at_levels` sets `brain_profile` and NOTHING ELSE per level, both
rungs seat the same `npc_pirate_admiral`, and the pool is therefore identical
(~100, back-computed consistently once the real cause below is accounted for).
The division looked like a measurement and was an inference over a quantity that
does not vary.

⇒ **MEASURED instead, by splitting each hit on whether its victim is a seat:**

| rung | dealt to a SEAT | dealt to a NON-SEAT body |
|------|-----------------|--------------------------|
| 5 | 16 (15%) | 89 |
| 9 | 51 (54%) | 44 |

Rung 5's fighters spend the duel hitting **summoned bodies**, not each other. It
is the rung that spams the summon: 71–84% of its move starts are `grapeshot` and
`call_the_shark` (27+9 of 51, 30+12 of 50), against 13–21% `grapeshot` at every
other rung, and its seat 1 uses only 7 distinct moves where rungs 3, 6 and 9 use
13–16. `A_REAL_FIGHT` is measuring exactly what it claims to and its verdict is
correct: the CPUs are not fighting *each other*.

⚠ **WHAT MISLED THE FIRST READING IS A REAL DEFECT, just not the metric's.**
`damage_by_move[slot] += hit.damage` keys on `hit.attacker` being a seat and
never looks at `hit.victim`, so the `[dealt]` line prints damage into summons as
damage dealt. It reported `105` for a duel in which `16` reached an opponent.
⇒ A reader takes that line as "how hard this seat is fighting" — the first
version of this row did — and at rung 5 it overstates by 6.6×.

**Next implementation:** two separable pieces, and the second is the row's real
subject.
1. ✅ **DONE.** `[dealt]` now prints both victim classes, and the `dealt > 0`
   PASSENGER ASSERTION beside it — not just the print — counted summon damage
   too, so a seat that never touched the opponent satisfied the half whose
   stated job is *"the exchange floor above can be carried by one seat alone,
   and that is exactly the state this half exists to refuse"*. It now counts
   seat-directed damage only. Measured across the five rungs, dealt-to-seat vs
   dealt-to-another-body per seat: rung 1 `83/24, 97/8`; rung 3 `69/60, 54/60`;
   rung 5 `6/40, 10/49`; rung 6 `46/0, 60/0`; rung 9 `35/34, 16/10`. ⛔ The
   predicate is not constant in either direction — rung 6 spends nothing on
   other bodies and rung 5 spends 89 of 105 — which is what makes the split a
   measurement rather than a relabelling. No rung changed verdict.
2. Rung 5's move selection is a [BRAIN](#brain--finish-truthful-fighter-attack-selection)
   defect, not a guard defect: a duelist policy that answers 71–84% of its
   decisions with two moves and lands 15% of its damage on the opponent is the
   "representative CPUs select from their authored menu across the intended
   difficulty ladder" acceptance failing at one rung. ⇒ Re-file the failure
   there; what stays here is the diagnostic.

⛔ **Do not fix this by lowering the floor or by normalising the metric.** Both
were the first version's instinct and both would have hidden a real brain defect
behind a guard change.

**Acceptance:** `[dealt]` distinguishes the two victim classes, and the guard
passes at all five published rungs at HEAD — by rung 5's CPUs fighting each
other, not by a threshold that moved.

### A2 — close the remaining projectile construction-identity hole — ✅ DONE 2026-09-16 (THIS ROW'S SCOPE; A2a/A2b/A2c are a different subject)

**Owner:** [`engine/projectile-contact-protocol.md`](engine/projectile-contact-protocol.md).

**Receipt.** The construction-identity hole is closed: the player-clone road is
deleted (`89d78a4a5`), and the five dynamic-mint fallbacks name their
construction road through `UnmintedBodyCensus`. Witness:
`a_body_the_sweeper_declines_to_identify_is_nameable`
(`game/ambition_app/tests/every_damageable_body_is_identified.rs`). ⚠ This is
not all of A2: the work frontier's
[A2a/A2b/A2c](engine/actor-monolith-work-frontier.md) (geometry, obstruction,
recipient naming) are a different subject. Two mint sites degrade on purpose
and belong to the `ItemCustody` inventory leg: `ambition_held_items`'s
thrown-item mint and `puppy_slug_gun`'s minion mint.

### A12 — finish move-contact attribution and reflection identity

⛔⛔ **THIS IS NOT THE ONLY `A12`.** The frontier's
[`A12`](engine/actor-monolith-work-frontier.md#a12-align-flow-validation-prepared-representation-and-execution-bounds)
is *"align flow validation, prepared representation and execution bounds"* — a
different subject, and as of 2026-09-17 a CLOSED one. Both live on
`MovePlayback`, which is how one label came to cover two packets without anybody
noticing. ⇒ Name the subject, never the bare number.

**Owner:** [`engine/authored-technique-admission.md`](engine/authored-technique-admission.md)
and combat/projectile occurrence identity.

**Current state:** ranged feedback carries `MoveOccurrence` end to end, and
melee stamps use the same occurrence authority.

⛔⛤ **AND THAT SENTENCE WAS TRUE OF TWO OF THE THREE CONTACT FACTS — CORRECTED
2026-09-19 BY READING THE WRITERS.** It said a projectile launched by move A
*"cannot be credited to whatever move happens to be playing when it lands"*.
`verdict_belongs_to` makes that so for `connected` and `blocked`, which are
written by `mark_move_playback_resolved_hits`. **`landed_hit` is written
somewhere else and asks nothing**:
`apply_feature_hit_events`
(`crates/ambition_platformer2d_actor_monolith/src/features/ecs/damage/mod.rs:914`)
sets it on the attacker's CURRENT playback for every `HitEvent` that reached an
actor or a boss, holding `event.attacker_move_instance` unread.

✅ **THE HALF THAT NEEDED NO RULING IS REPAIRED — 2026-09-19.** An event
carrying `Some(other_instance)` names its author, so crediting the live move
contradicted the 2026-09-10 ruling rather than waiting on `Q101`; that case is
refused now and witnessed by
`an_outcome_naming_another_occurrence_credits_no_move`. ⇒ What is left of this
row is the `None` case — `blink`, `dive`, `mark_recall`, `empowerment` — and it
is the product ruling `Q101` owns, with the authored flow that already waits on
`Overlapped` recorded there.

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

✅ **AND CONTACT ATTRIBUTION IS CLOSED, 2026-09-19.** All three halves:

1. Reflection — `intercept.rs` drops `FiredByMoveInstance` with the ownership
   change, so a reflected shot stops claiming its original launcher.
2. An outcome naming a DIFFERENT occurrence credits no move. That needed no
   ruling: the event says whose it is.
3. An outcome naming NO occurrence credits no move either — **ruled
   2026-09-19**: an ability contact is INDEPENDENT BY DEFAULT and satisfies a
   move's `Connected`/contact condition only with explicit provenance. `None`
   must not mean *"credit whatever is playing now"*. An ability designed to
   count toward its launcher threads the occurrence; there is no implicit road
   back. See [`maintainer-decisions.md`](maintainer-decisions.md).

⛔⛤ **AND THE FIRST TEST TO GO RED WAS A FIXTURE DESCRIBING A ROAD PRODUCTION
DOES NOT HAVE.** `a_player_slash_folds_the_struck_target_onto_the_move_accumulator`
sent a `HitSource::Melee` event with a `MovePlayback` attacker and
`attacker_move_instance: None`, then asserted the dedup ledger folded. The
production melee road writes `move_instance.map(..)` on every strike
`hitbox/mod.rs` resolves — so the fixture's shape was one only the implicit
road could produce, and it now names the occurrence as production does. ⇒ A
fixture that keeps a shape production cannot produce is testing a road that no
longer exists, and it reads as a regression when the road is removed.

**Blocked by:** nothing.

**Acceptance:** ✅ MET — late projectile/melee feedback, reflection and
independent ability contacts cannot credit the wrong move occurrence, including
across an idle gap and rollback. Witnessed by
`an_outcome_naming_another_occurrence_credits_no_move` and
`an_unclaimed_outcome_credits_no_move`
(`features/ecs/damage/tests.rs`), plus
`a_move_occurrence_reaches_the_same_number_with_and_without_a_rewind` for the
rollback half.

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

### AUTHORITY-POLISH — one owner per mechanical fact, and no mirror in the rollback kernel

**Owner:** the architecture-completion campaign (opened 2026-09-23). Durable
evidence for rows marked `W0xx` stays in
[`architecture-warts/README.md`](architecture-warts/README.md) until the row
closes; this row is the execution order. It does not displace the rows above
(ID-PEER, A4, the rollback rows) or C03/C06/C07 in
[`consolidation-plan.md`](consolidation/consolidation-plan.md); it is the
authority-polish lane beside them.

**Order of the open AP rows (from the review of `6cacbdd..bab8612`, 2026-09-26).**
Finish the boundaries that later work builds on before the local defects:
(1) AP76's admission boundary: registration conflicts fail closed, and
`hurtbox_insets` content that the validators accept always lowers (AP79);
(2) AP78, capability-owned facet lowering, before any other facet reaches
`PackCast`; (3) the rest of the seat-level portal consequences move into the
runtime beside AP77's turn-around (emission guard, held-input warp,
`BodyTeleported`, trail seam); (4) AP8 and AP34 as DISCOVERY passes: stop the
census at the first real duplicate or fallback, finish that migration, then
continue. Do not try to classify every name. (5) AP56(2). (6) Reassess: if two
discovery passes find little, move the campaign to dependency cycles and
monolith decomposition, and extract only where that gives a fact a clearer
owner or removes a cycle. AP70 was already fixed when this order was set.

**Where the order stands (2026-09-27).** Steps 1–5 are done: AP79 (1), AP78 (2), AP80 and AP84 (3), two AP8 discovery passes and one AP34 pass (4, one finding: AP81), AP70 and AP56(2) (5). Step 6, reassessed: `measure_kernel_module_graph.py --scc` still gives the 6-module kernel (`abilities features items projectile session world`) plus `assets ↔ character_sprites`. Every leg of the kernel has a recorded verdict in AP14. The 2-cycle is the asset aggregator loading a domain's sheets while the domain reads the aggregator's catalog ids; cutting it would be a graph-only move, which the criterion rejects. So the campaign spent step 6 on game expression, where the discovery passes pointed: AP82 (a form's traits are its row's), AP83 (a persona's ring shield is its row's), AP85 (a transformation's beat travels with its request).

**The question every item answers:** what owns this semantic fact? Target shape:
authoring/ruleset → preparation → one construction or transition → one
canonical live authority → disposable read models → presentation. Prefer
deleting the field, the writer and the reconciler over a stronger sync.

**Order** (two rollback truths first, then construction repair, competing
writers, fail-open mechanics, wrong owners, dead projections, diagnostics):

| # | Item | State | Semantic fact → single authority | Evidence / next step |
|---|---|---|---|---|
| AP3 | Dead rollback projections | ✅ first four DONE (this campaign; see commit) | `BossPatternTimer` (W013; the brain's `BossPatternState.pattern_timer` owns it, anim reads the brain), `ActorStatus::ai_mode` (W014) with the evaluator that only fed it and `provoke`'s `chase` flag, `CombatTuning::attack_cooldown_mult` (W018), `ActorFireRequest::speed` (W024; the resolved `RangedActionSpec` owns launch speed). Schema v206. | Reviewed and kept: `BossPhase` is the boss's previous-tick liveness, the edge latch that starts its death animation. `BrainProfile::attack_cooldown_mult` is authored and read by nothing. `BodyMelee::begin(.., cooldown)` has no production caller, so the melee cooldown floor is never armed — decide whether that is intended before deleting. | <!-- cite-ok: the deleted fields this row closed -->
| AP8 | Bounded C07 audit | ✅ DONE 2026-09-27 (bounded) | Optional-authority reads of the mechanical and session authorities are classified per site (optional capability: keep; required authority absent: queue; global fallback: suspicious), and the discovery passes that followed found one duplicate, AP81. Witness: `an_undriven_home_body_in_a_door_is_not_crossed_for_anyone`. | — |
| AP9 | Stale architecture docs | CONTINUOUS | W020 (`SimDt`) closed: `SimDt` is registered `declare_rollback_derived_resource`, a derived mirror rather than a second authority. | Remove closed wart rows as items land; keep one-line receipts only where another row depends on them. |
| AP10 | Control state stored twice | ✅ DONE 2026-09-23 | `ScriptedControl` and `TemporaryControl` are deleted (schema v208), so `ControlHolds` and `ControlClaims` are the only control-state authorities. Witness: `a_second_seat_and_a_possessed_body_both_collect`. | — |
| AP11 | Per-tick mirrors in canonical rows | ✅ DONE 2026-09-24 (v221) | Per-tick mirrors of an owner's value on the session root (`ActiveRoomMetadata`, `RoomMusicRequest`, `SwitchOn` and the rest) are deleted; readers ask the owner, for example `RoomSet::active_metadata()`, and a body reads its own `ResolvedMotionFrame`, not the primary body's gravity. Witness: `with_no_zones_a_body_takes_the_ambient_not_the_primary_bodys_gravity`. | — |
| AP12 | Dead or constant rollback state | ✅ DONE 2026-09-24 | Dead or constant rollback state is deleted (`LocalPlayer`, the `PlayerSlot` component, the encounter music mirror (W021), `BodyOffense`, the melee pending axis and others), and each kept row has a recorded reason. | — |
| AP13 | Presentation state in the rollback kernel | ✅ DONE 2026-09-24 | Two presentation fields left the rollback kernel (the blink camera target and the encounter placement id); `BodyAnimFacts`, `BossDeathAnimation`, the sprite size and offset rows and `PickupArt` stay canonical because sim events arm them or a re-created entity needs them. | — |
| AP14 | Semantic actor-monolith SCC decomposition | CONTINUOUS (Jon, 2026-09-24) | The largest textual SCC is 10 modules — `abilities avatar character_runtime construction control features items projectile session world` — plus `assets ↔ character_sprites` (measured 2026-09-24 at `2c95a83b9`). Re-measured after AP15: unchanged (10 + 2). 2026-09-24: the item domain installs its own durable-save adapters (`items::persist::install_item_durable_horizon`) into slots `session::durable_horizon` publishes (`DurableHorizonSet`, `DurableRestoreSet`); the runtime's `DurableSaveHorizonPlugin` composes both. `session→items` 19→14 refs, SCC unchanged. The remaining `session→items` refs are the minted-item descriptors (`MintedItemBaseline`, `OwnedItemsBaseline`, `ItemCheckpointRestoreInputs`) that `CandidateDurableHorizon`, the checkpoint pin and teardown carry: open question whether runtime-mint description is item knowledge or occurrence-lifecycle knowledge — if the latter, it moves to the lifecycle owner and the edge inverts for a reason. 2026-09-24: possession moved from `abilities::traversal` to `control::possession` (with `PossessionPlugin` replacing `AmbitionAbilitiesPlugin`, which only initialized its resources). It is a seat redirect — control authority, as the `abilities` module doc already argued — and it was the only `abilities`↔`control` reference in either direction. **SCC 10 → 7**: `avatar`, `character_runtime` and `control` left; the cycle is now `abilities construction features items projectile session world` (+ `assets ↔ character_sprites`). Remaining single-edge clues then: `abilities→features` (the puppy-slug gun, rejected as a graph-only inversion), `features↔projectile`, `construction→world`. 2026-09-24: the placement-lowering context (`ActorPlacementContext`, `LoweringCtx`, `LoweringFn`, `PlacementLoweringRegistry`) moved from `world::placements` to `construction::placements` — its commit facts are construction's `PersistedFates` and construction's recipes are its lowerers; `world` only stages rooms with it. **SCC 7 → 6**: `construction` left; now `abilities features items projectile session world`. `features↔projectile` is judged genuine and left: perception reads a shot's `ProjectileAllegiance`, and projectile contact asks the feature predicates (`projectile_reaches_breakable`/`_boss`) — each consumer owns the question it asks. 2026-09-24: the feature collision-overlay rebuild (`rebuild_feature_ecs_world_overlay`, which reads breakables and pogo contributors, is installed by `features` and was re-exported by it) moved from `world::overlay` to `features::ecs::world_overlay`; `features` no longer re-exports `world::rooms::LastConstructionVerification` (readers name `world::rooms`). `features→world` is gone; SCC unchanged at 6 — `world→features` (room staging drives the feature construction plan) is the remaining leg, and whether that plan belongs to `construction` is the next question. 2026-09-24, the remaining SCC legs walked and judged genuine (no move): `world→session` is a door recording its `LifecycleIntent` into session's commit slot, and `session→world` is reset and setup republishing the room — session owns lifecycle arbitration, rooms own publication. `world→features` stays: room staging hands feature spawning (`features::ecs::spawn`) to the domain that knows what the entities are, and moving that plan to `construction` would only add `construction→features`. `session→items` (14) stays: `AcceptedRestore` is ONE value per checkpoint operation — its checksum, the abandoned-operation identity and post-apply verification all cover the item half — so splitting the pin into an item-owned keyed resource would fragment that identity; `CandidateDurableHorizon` carries `MintedItemBaseline` as one value for the same reason (review finding 2), and teardown's reset is the session-scope census by design. `items→session` is the item domain plugging into session's published slots. ⇒ The 6-module SCC's only leg judged misplaced is `abilities→features` (the puppy-slug gun), and inverting it only for the graph is rejected. The 6-module SCC is left as the residual kernel unless a new ownership question appears. `shrine` left through a real ownership move (checkpoint restore → session lifecycle); `avatar` and `character_runtime` later joined, so the count is not monotone. Single-edge cut scores (`abilities→features` 10→6, `control→abilities`, `avatar→control`, `character_runtime→avatar`, `features→projectile`) are CLUES, not objectives: `abilities→features` is the puppy-slug gun asking the actor domain to spawn a minion, and inverting it only for the graph is rejected ([`actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md), [`actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md)). | Every AP item asks: does the dependency this seam touches exist because the consumer owns the knowledge, or because an authority/adapter is misplaced? When the owner is clear, move state + behavior + installation/lifetime together; delete the obsolete edge (no callbacks, no compat re-exports). After each coherent migration run `python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80` and record what fell out here. Target: a small residual body-integration kernel whose remaining cycles are genuinely inseparable simulation, not a number. Queue new decomposition rows as cleanup reveals misplaced responsibilities. |
| AP15 | Morph Ball entitlement | ✅ DONE 2026-09-24 (v215) | `BodyModeCapabilities` is deleted; `AbilitySet` carries `crouch`, `climb` and `morph`, and Morph Ball is the Ambition provider's home-body grant. Witnesses: `mary_o_it::morph_ball_is_not_hers` and `app_it::morph_ball_is_the_protagonists`. | — |
| AP17 | STRETCH A: one rollback-canonical time authority | ✅ DONE 2026-09-24 | `WorldTime` is the one rollback-canonical sim clock (one production writer, `refresh_world_time`), and sim systems read it instead of `Res<Time>`. Guard: absence contract `the-sim-clock-is-world-time`. | — |
| AP18 | Control gate was player-only | ✅ DONE 2026-09-24 (one residue recorded) | `gate_worn_player_control` filtered `(With<PlayerEntity>, With<WornCharacter>)`, so a body driven by possession or a second seat kept a raw frame: no technique edge (content grants techniques to the `ControlledSubject`), and no stripping of unowned verbs (guard, grab, attack). Renamed `gate_body_control`; the filter is gone and every body is gated by its own scheme. Census before the change (a probe beside the gate, app_it + mary_o/sanic/smash/twintrack, ~1.7M non-player body-ticks): autonomous bodies changed 0 times, and only DRIVEN bodies differed — twintrack's lab twin (a stray `attack_axis`) and a possessed boss. Witness: `a_driven_actor_outside_the_player_population_is_gated_by_its_own_scheme` (red with the `PlayerEntity` filter restored). | ✅ 2026-09-24, the possessed-boss residue: a boss's `ActionSet` (`special: None`) and profile-keyed moveset declare no `attack`/`special` verb, so a possessed gnu_ton rider's scheme was Jump, Ranged and Interact, and so was its prompt (measured). The boss road now inserts `ActorTechniques(possessed_boss_techniques(..))`: Attack and Special as `Technique` gates whose ids are the moves the neutral presses resolve to, by the same helpers `possessed_attack_choice` uses (the rider: Hand Sweep, Apple Rain). Not moveset verbs: the boss carries `ResolvedAttackGesture` and `BodyActionBuffer`, so `trigger_moveset_moves` would start a second move for one press, from t=0, where the possessed road starts at the strike edge. The gate clears the raw press as it did when the slots were absent, so behaviour is unchanged. No schema change: `ActorTechniques` is a declaration (waived in `rollback_coverage`, which found it in `mockingbird_arena`), and the edges row is derived. Witnesses: `a_possessed_boss_prompt_shows_the_attack_and_special_it_fires` (poisoned without the insert: the prompt reverts to three slots) and `the_declared_possessed_actions_are_the_moves_the_presses_fire` (poisoned to ignore the verb map: `hand_slam` against `hand_sweep`). ⚠ Residue: the boss tick does not read the gate. It builds its own frame from the seat's `SlotControls` (`tick_player_brain`) and chooses from that, so the choice does not depend on gate order, but it is a second input road beside the gate. Reading the routed technique edge instead needs the edge to carry the aim: the `Technique` arm clears `attack_axis`, which the directional choice reads. ⚠ A boss that holds an item: the `Technique` arm clears the Attack press where the absent slot kept it for the item; no boss holds one today. |
| AP19 | Unauthored bodies are given a capability default | ✅ DONE 2026-09-24 (ruling: the provider declares it) | The generic actor constructor confers no abilities; a body's capabilities come from its authored definition or its provider's declared default. Witnesses: `an_anonymous_npc_is_built_with_no_abilities` and `a_prepared_npc_without_a_blueprint_spawns_with_its_authored_abilities`. | — |
| AP28 | A provoked body is handed an engine-invented fighter (W026) | ✅ DONE 2026-09-25 (ruling: ruleset/content owns it) | The engine-invented provoked policy is deleted (schema v229): a provider declares its default provoked profile on its catalog fragment, and a body with no policy cannot be provoked. Witnesses: `a_silent_character_is_provoked_into_its_providers_declared_policy` and `a_body_with_no_provoked_policy_cannot_be_provoked`. | — |
| AP29 | The actor's construction record was also its live policy (W017) | ✅ DONE 2026-09-25 (v231) | The live brain policy is its own rollback component, `ActorPolicy(BrainProfile)` (`actor.policy`), and `ActorConfig` is construction input only. Witness: `a_bodys_policy_is_rollback_state`. | — |
| AP30 | A catalog row was a character nothing prepared (AP19/AP28 reach) | ✅ DONE 2026-09-25 | The preparation barrier prepares every catalog row, not only authored definitions, so provider declarations reach catalog-only characters. Witness: `every_hall_body_is_provoked_into_its_policy_or_stays_peaceful`. | — |
| AP31 | A character's provider had two answers | ✅ DONE 2026-09-25 | A character's provider is read from its prepared definition only; `CharacterCatalogOwners` is read only by assembly and by the parity audit's cross-check. | — |
| AP32 | A provoked mind was lowered from the construction record (W017) | ✅ DONE 2026-09-25 | The stored patrol and chase speeds on `ActorTuning` are deleted, and brain builders read the policy they are given, so a provoked mind moves at its provoked policy's pace. Witness: `every_hall_body_is_provoked_into_its_policy_or_stays_peaceful`. | — |
| AP33 | The boss encounter phase was stored twice in one component | ✅ DONE 2026-09-25 (v232) | The copied boss encounter phase and its sync system are deleted; readers call `BossEncounter::encounter_phase()`, which reads the encounter state. | — |
| AP34 | `sync_*` census (the campaign's classification rule) | ✅ DONE 2026-09-27 (bounded; d32b5e8bb, 341d2033f) | Every production `fn sync_*` is classified by reading it; each one left is a projection or a real transition with its reason recorded, and a new `sync_*` is judged by the same rule when it is written. | — |
| AP36 | A second collision world built from LDtk plugin entities | ✅ DONE 2026-09-25 | The LDtk runtime spine no longer builds collision indices from `bevy_ecs_ldtk` entities; the world-IR collision world is the only collision authority (ADR 0021). | — |
| AP37 | An encounter's room was its id | ✅ DONE 2026-09-25 | `EncounterSpec.room_id` records an encounter's room, and the lifecycle, the reward chest and switch targeting key on it. Guards: `a_fight_cleared_in_one_room_gives_no_chest_in_another` and `an_unnamed_target_is_the_encounter_authored_in_the_active_room`. | — |
| AP45 | A fighter's moves are authored in Rust and shipped as RON | ✅ DONE 2026-09-26 | A fighter's RON file under `assets/data/movesets/` is the only source of its moves (the 17 Rust tables and the exporter are deleted), and a variant can `borrows` an archetype's table. Guards: `a_borrower_plays_its_archetypes_timings_under_its_own_name` and `a_chain_of_borrows_is_refused`. | — |
| AP48 | The collision overlay is room-scoped and does not know its room | ✅ DONE 2026-09-26 | The collision overlay leaves with the room it describes, so a commit tick does not compose the new room's geometry with the old room's overlay. Guard: `the_collision_overlay_leaves_with_the_room_it_describes` (app_it). | — |
| AP56 | The overlay's contributors are unordered among themselves, and a rebuilt body lags one tick | ✅ DONE 2026-09-27 | A composed world does not depend on the order its overlay blocks were contributed, and the one-tick lag of a rebuilt body was in the instrument (an arm's age is now counted from the step that built it). Guard: `a_composed_world_does_not_depend_on_the_order_its_blocks_were_contributed`. | — |
| AP57 | A brained body is built without its senses, and a system repairs it | ✅ DONE 2026-09-26 | A brained body is built with its senses; the per-tick construction repair is deleted, and the perception extent is read from the session, not copied onto each body. Guard: `a_session_that_cannot_decide_senses_takes_the_body_out_of_the_decision`. | — |
| AP58 | A game states its cast twice: catalog rows in a Rust string, and a Rust registration loop | ✅ DONE 2026-09-26 | A game states its cast once, as catalog rows in its content pack, and the Rust catalog strings and per-character registration loops are deleted. Guard: `the_smash_demos_cast_is_prepared_from_its_pack_as_the_rust_registration_built_it`. | — |
| AP62 | Three passes opt bodies into portal transit on first sight, and two games reconcile one transit | ✅ DONE 2026-09-26 (341d2033f) | The per-tick passes that opted bodies into portal transit on first sight are deleted, so one hosted transit no longer has two games reconciling it. | — |
| AP66 | Fourteen Ambition creatures are a Rust file each that states only what their catalog row could say (AP58 step 3, first cut) | ✅ DONE 2026-09-26 (bc9c18889) | Fourteen Ambition creatures are catalog rows, not Rust files. Guard: `a_row_that_states_its_body_is_built_with_that_body`. | — |
| AP67 | Ten more Ambition creatures are Rust files, and nine of their rows name an action set the file overrules (AP58 step 3, second cut) | ✅ DONE 2026-09-26 (7431f4106) | Ten more Ambition creatures are catalog rows, and no row names an action set that a file overrules. Guard: `a_row_that_states_its_body_is_built_with_that_body`. | — |
| AP68 | A scripted boss's beat runs on two clocks, so a phase change eats the next move's windup | ✅ DONE 2026-09-26 | A scripted boss's beat waits for a committed strike before a new phase's first telegraph, so a phase change does not eat the next windup. Guard: `a_new_phase_waits_for_a_committed_strike_before_its_first_telegraph`. | — |
| AP69 | Seven more Ambition creatures are Rust files, because a row cannot say mount, mass, dream seed, held item or provoked policy (AP58 step 3, third cut) | ✅ DONE 2026-09-26 (8da7fbe42) | A catalog row can state mount, mass, dream seed, held item and provoked policy, so seven more Ambition creatures are rows. Guard: `a_row_that_states_its_body_is_built_with_that_body`. | — |
| AP70 | A boss decides its move before the tick's damage ends its phase, so a new phase opens with a one-tick ghost of the old phase's move | ✅ DONE 2026-09-26 | A phase change ends the old phase's windup on the same tick, so a new phase has no one-tick ghost of the old move. Guard: `a_phase_change_ends_the_old_phase_s_windup_on_the_same_tick`. | — |
| AP71 | The last three Ambition creatures are Rust files, because a row cannot say a weapon (AP58 step 3, last cut); `authored/` is gone | ✅ DONE 2026-09-26 (e2aea8bf5) | A catalog row can state a weapon, so the last Ambition creatures are rows and `authored/` is gone. Guard: `the_weapons_leave_the_body_where_they_are_drawn`. | — |
| AP76 | The player robot, Ambition's protagonist, was the last Ambition character built in Rust (`player_robot_lineage.rs`), beside three rows that said only its name, sheet and lines | ✅ DONE 2026-09-26 | The player robot is catalog rows like every other Ambition character; its Rust lineage builder is deleted. Guard: `the_robot_authors_its_verbs_rather_than_taking_a_match_s_word_for_them`. | — |
| AP77 | The portal core decided who turns around from `PlayerEntity`, so a possessed body kept its facing and the home body turned (review of `ac07b3e..6cacbdded`, P2) | ✅ DONE 2026-09-26 | The portal core writes no facing; the body a seat drives turns around, not the `PlayerEntity` population. Guard: `only_the_driven_body_turns_around_through_a_same_wall_pair`. | — |
| AP78 | `PackCast::register` names the smash-fighter capability, and the Smash demo interprets the same facet a second time | ✅ DONE 2026-09-26 | The smash-fighter capability has one typed owner in `ambition_characters::smash_fighter`, and no other code reads its facet. Guard: absence contract `the-smash-fighter-facet-is-read-only-by-its-owner`. ⚠ Reopened by `7b5d3f56c` (`MoveDamageSource::SmashFighterFacet` in `MatchRules`, `SeatTerms` and `WornKit`); closed again 2026-09-30: the owner folds its move damage under its own `DamageScale` (`smash_fighter::FIGHTER_DAMAGE`), and the match, the seat and the kit resolver read only a scale (`ambition_characters::move_damage`). The contract is green again; `the_robot_deals_health_damage_at_home_and_percent_on_a_smash_stage` still holds 1/4 at home and 3/15 seated, re-wear included. | — |
| AP79 | AP76 moved the robot onto a registration road that dropped identity conflicts, and the new `hurtbox_insets` field accepted rows its lowering panics on (review of `6cacbdd..bab8612`) | ✅ DONE 2026-09-26 | A character id that another provider registered stops the composition, and `hurtbox_insets` rows that the validators accept always lower. Witness: `a_character_id_another_provider_registered_stops_the_composition`. | — |
| AP80 | A portal transit's seat consequences were Ambition content, so a Sanic room guarded its player's input when Ambition hosted it and not when it ran alone | ✅ DONE 2026-09-26 | A portal transit's seat consequences (emission guard, held-input warp, `BodyTeleported`, trail break) belong to the runtime, so a Sanic room behaves the same hosted and standalone. | — |
| AP81 | The combat rules' undeclared baseline was written three times: `resolve(None, ..)`, a hand-listed `Default`, and the projection's own fallbacks | ✅ DONE 2026-09-27 | The undeclared combat baseline is stated once, by `resolve(None, ..)`; the hand-listed `Default` copy is deleted. Guard: `a_world_without_its_baselines_resolves_as_one_with_them_untouched`. | — |
| AP82 | A super form's traits were a Sanic system keyed on the form's id | ✅ DONE 2026-09-27 | A super form's traits are a character fact (`Empowerment` on the definition), not a Sanic system keyed on the form's id. Guards: `a_worn_form_and_a_timed_grant_empower_a_body_independently` and `the_super_form_s_traits_are_stated_by_its_row`. | — |
| AP83 | Which characters' rings absorb a hit was a list of Sanic ids in Rust | ✅ DONE 2026-09-27 | `wallet_shield` is a character fact on the catalog row, and the Sanic rule asks the worn character's prepared row. Guard: `the_ring_shield_is_stated_by_the_worn_row`. | — |
| AP84 | What a body carries through a portal was mapped by Ambition content, so a game without it lost a fling | ✅ DONE 2026-09-27 | What a body carries through a portal (a fling's run floor, a shot's acceleration) is mapped by the runtime's `portal_body` module for any game. Guard: `portal_schedule_plugin_carries_a_fling_for_any_game`. | — |
| AP85 | A transformation's beat policy was a second, unrolled component beside its rolled-back request, and Sanic repaired it onto every body on first sight | ✅ DONE 2026-09-27 | A transformation's beat policy rides in its rolled-back request (schema v238), and the Sanic first-sight repair is deleted. Guard: `the_super_form_s_traits_are_stated_by_its_row`. | — |
| AP93 | A hosted game stated its rooms twice, once for its declared rules and once per system, and the two drifted | ✅ DONE 2026-09-27 | A hosted game states its rooms once, as its `RulesScope`, and its systems take their gate from that scope. Guard: `a_standalone_mary_o_refuses_a_weaker_form_pickup`. | — |
| AP101 | Mary-O kept her room's goal pole and its destination as resources copied from the room | ✅ DONE 2026-09-27 | Mary-O reads the goal pole and the destination from the active room's authored data; the copied resources are deleted. Witness: `the_flag_is_the_pole_of_the_room_she_is_in`. | — |
| AP102 | `GoverningRules` read "no session" as "a live untagged room" | ✅ DONE 2026-09-27 | `ActiveRoom { NoRoom, Untagged, Mode }` states the three facts, and `RulesScope::governs(room)` is the one table, so a rule and its gate agree about no room. Witness: `no_room_is_not_an_untagged_room`. | — |
| AP104 | Smash's respawn interval was a process-wide resource | ✅ DONE 2026-09-27 | A stocks match's respawn interval is a match rule (`MatchRules::stock_respawn_seconds`) carried by each fighter's stocks, not a process-wide resource. Witness: `a_stocks_match_seats_fighters_with_its_own_respawn_interval`. | — |
| AP108 | Mary-O left each level by her own copy of the engine's departure loop, on an owner that also carried the engine's | ✅ DONE 2026-09-27 | Mary-O leaves a level through the engine's `Departure`, not her own copy of its loop. Witness: `a_read_tally_asks_to_leave_once_and_its_own_replay_starts_the_lap`. | — |
| AP109 | Two games each kept "which room is my mode in" on their own owner, one as an index and one as an id | ✅ DONE 2026-09-27 (434574577) | Every mode owner carries the engine's `ModeVisit`, and one engine system records which room the mode is in and how it arrived. Witness: `a_mode_owner_is_told_when_it_arrives_in_a_room`. | — |
| AP128 | GNU-ton's arena facts are written by `adopt_gnu_ton`, a tick after the bodies are built | ✅ DONE 2026-09-28 (d8c8baf37; map assets 6fead7f) | GNU-ton's, the FSM's and the cut-rope arena's facts are written in the construction transaction from data (a boss birth kit, `unmirrored`, the placement's depth plane, `rider_conducts`, an authored encounter script), and the three adopt/setup systems are deleted. Guards: `the_fists_are_built_as_the_scholars` and `the_behemoth_and_its_fight_are_built_whole`. | — |
| AP133 | Every Mary-O level opened with the card "WORLD 1-1" | ✅ DONE 2026-09-28 | `RoomMetadata::title` is an authored level field, and Mary-O's card reads the active room's title. Guard: `each_level_opens_with_its_own_title`. | — |
| AP137 | A boss script's prop and member names were checked only when a beat fired, after the beat was consumed | ✅ DONE 2026-09-28 | A boss script's prop and member names are resolved when the script is prepared, so a room whose script names a prop it lacks is refused at construction. Guard: `a_boss_script_naming_a_prop_the_room_lacks_fails_the_room_while_it_is_whole`. | — |
| AP140 | Speed shoes save the authored momentum numbers and write them back | ✅ DONE 2026-09-28 | Speed shoes are a momentum boost that lifts the run cap; nothing saves or writes back the authored momentum params. Guard: `a_momentum_boost_lifts_the_run_cap_without_writing_the_params`. | — |
| AP146 | Three games each loaded their own prop sheets from a per-frame latch | ✅ DONE 2026-09-28 | A game registers a prop kind's sheet with the engine, and registered prop sheets load with the first `GameAssets`; the three per-frame loaders are deleted. Guard: `registered_prop_sheets_load_with_the_first_game_assets`. | — |
| AP147 | A hold suspends gravity by writing the victim's `gravity_scale` and saving a copy to restore | ✅ DONE 2026-09-28 (mount and capture; the Gnu-ton fists are handed off below) | AUTHORITY RECONCILIATION, found while closing AP140 (the same save-and-restore shape). `ActorSurfaceState::gravity_scale` is the authored answer to "how strongly does gravity pull this body", and two relationships overwrite it: a capture writes `0.0` every tick of the hold and keeps `CapturedBy::prior_gravity_scale` to write back in `release_capture` (`crates/ambition_combat/src/capture/systems.rs`), and a mount writes `0.0` on the rider and restores `baseline.gravity_scale` on dismount (`crates/ambition_mount/src/lib.rs`, the dismount and the rider-respawn arms). The kernel already names the problem: the doc of `AxisManeuverState::gravity_modifier_scale` (`crates/ambition_platformer2d_core/src/movement/model.rs`) says "three domains already save-set-restore that field (capture, mount, body seed)", and `integration.rs` folds its other gravity factors as a product precisely so no writer saves a prior. Direction: a held body's gravity is a fact of the hold, not of the body. Both relationships already have the right marker or can take it: a mount puts `PoseOwnedExternally` on its rider, and `step_body` already passes `ctx.pose_owned_externally` to the kernel. Have the kernel apply no gravity while `pose_owned_externally` (the pose is the constraint's, so gravity has nothing to move), give a captive `PoseOwnedExternally` for the hold as the mount does its rider, and delete the `0.0` writes, `CapturedBy::prior_gravity_scale` and the mount's gravity restore. Measure first: (1) the home body road passes `false` for the held flag (`avatar/body_integration.rs`), so a captured HOME body needs the marker read there as the actor road does; (2) whether anything reads `gravity_scale == 0.0` as "this body is held" (the body seed treats `<= 0.001` as aerial, `ambition_body_seed/src/lib.rs`), which a hold would then stop implying. ⚠ MEASURED 2026-09-28, and it is the blocker: two brain inputs read the zero as a locomotion fact, not a hold fact. `actor_aerial: body.surface.gravity_scale <= 0.001 || body.flight.fly_enabled` (`features/ecs/actors/update.rs`) decides whether a brain steers a 2D `velocity_target`, and `aerial: body.surface.gravity_scale <= 0.001` (`features/ecs/perception.rs`) is its perception twin. A rider in the saddle is "aerial" today only because the saddle zeroed its scale, so a CPU rider on a flying mount (Smash's pirate on the burning shark) may steer vertically BECAUSE of the write. The zero cannot go until a rider's aerial fact comes from its mount's locomotion (the `RidingOn` relation) instead of from its own scale. Resolve frames also apply the scale only in the actor arm (`gravity/resolve.rs`), which is where a held body's zero response belongs. Guard to write: a captured flying body (authored `gravity_scale` not 1.0) released after N ticks keeps its authored scale, and no system wrote the field during the hold. ✅ MOUNT HALF DONE 2026-09-28. The blocker was resolved first: `steers_in_flight(surface, flight)` (`features/ecs/actors/update.rs`) is the one answer to "does this brain steer a 2D `velocity_target`", and `tick_actor_brains` asks it of the MOUNT when the body has `RidingOn` (a rider steers the pair, and the pair flies when the mount flies). Its perception twin `PerceptionBody::aerial` / `SelfView::aerial` had NO reader, and a different rule (no `fly_enabled` half); both fields are deleted. Then the hold: the frame resolver (`gravity/resolve.rs`) gives a body with `PoseOwnedExternally` a zero response, so the saddle's two `gravity_scale = 0.0` writes, the dismount restore, the mount-death restore and `SpawnBaseline::gravity_scale` (read only by those two restores) are deleted. Schema 255 -> 256. Guards: `gravity::resolve::tests::a_held_body_resolves_no_pull` (control body beside it); app_it `pirate_sky_crew::a_ride_holds_its_rider_without_writing_its_gravity` (poison: drop the held arm → "a raider held in the saddle still resolved a pull"); `pirate_sky_crew::a_rider_on_a_flying_mount_steers_in_flight_from_the_first_tick` (poison: ask the rider about itself → "npc_pirate_raider hung still ... 517v0: 60"; MEASURED: the engaged Skirmisher steers `velocity_target` either way, only its idle sweep reads the fact, which is why the older cove test stayed green under that poison). ✅ CAPTURE HALF DONE 2026-09-28: the resolver reads `CapturedBy` as a hold too, so `pose_captives`' per-tick `0.0`, `release_capture`'s restore and `CapturedBy::prior_gravity_scale` are deleted (and the `ActorSurfaceState` columns of the four capture systems that only carried them). `CaptureParticipant::surface` stays, now documented as what it is: the component that puts a body in the resolver's actor arm, without which a captive would be held under full pull. Timing is unchanged: the frame is resolved at the start of a tick, and both the old write and the relation's insert/remove land before the next resolve. Schema 256 -> 257. Guards: `a_held_body_resolves_no_pull` now spawns a captive beside the rider (poison: drop the captive term → "captive: zero pull"); `capture::systems::tests::a_hit_captor_drops_its_captive_and_leaves_its_gravity_alone` (a captive floating at `0.25` keeps `0.25`). HANDOFF, not done: the Gnu-ton conductor's fists (`bosses/gnu_ton/conductor.rs`) write `0.0` while posed and `1.0` (their authored walking scale) on release. The vocabulary exists (the FSM conductor in `bosses/fsm/conductor.rs` inserts `PoseOwnedExternally` on its god), but moving the fists onto it also changes what a HIT does to a posed fist: the kernel stages a tumbling launch until the hold ends, so a fist launched mid-conduction would fly when the gnu dies. Measure whether fists take tumbling launches before switching. |
| AP148 | Mary-O kept a per-frame fallback that published three enemy sheets the engine already owned | ✅ DONE 2026-09-28 | Mary-O's per-frame fallback sheet loaders for three enemies are deleted; the engine's declared-character road publishes those sheets. Guard: the Mary-O app and `app_it` suites stay green with the road gone. | — |
| AP54 | A move file cannot take single moves from another table | ✅ DONE 2026-09-26 | A move file can take single moves from another entity's table by id, with the same refusals as `borrows`. Guard: `a_taker_plays_the_taken_move_as_its_owner_authors_it`. | — |

**Discovered while working (record, then return to the order above):**

- 2026-09-23 rollback-row census (363 non-derived component/resource rows at
  `8ee4936ed`; 44 flagged, grouped as AP10–AP13 above; 319 unflagged by an
  automated pass plus ~75 read by hand — "no finding", not "proven clean").
- `WorldTime` is itself recomputed from `Time.delta` × `ClockState.time_scale`
  at the head of each step — the stretch goal's canonical clock; not a defect,
  noted so a later census does not re-flag it.

**Acceptance for the lane:** no mechanical fact has two mutable canonical
owners; rollback rows are authorities, not projections; construction publishes
no plausible-but-incomplete object; required mechanical policy does not fail
open. Close with a fresh census rather than a checked list.

## P1 — ownership, composition and iteration

### BOSS-REPLAY-RETRACTION — a replay that un-defeats a boss un-defeats it for every family

**Owner:** the generic boss-progress road (`crates/ambition_boss_encounter`)
jointly with the save's replay policy
(`crates/ambition_persistence/src/save_data.rs`,
`every_durable_family_says_whether_a_replay_retracts_it`).

**Ruling (Q51 and Q56, 2026-10-01, [`maintainer-decisions.md`](maintainer-decisions.md)):**
if a replay or rewind makes a boss undefeated again, the consequences its
defeat created after that point are undone too, for every boss family. A
rollback boundary that does not yet enforce this is a known issue, not an open
question.

**Current state, measured 2026-10-01.** One family retracts:
`reset_cut_rope_attempt_on_replay` sets each cut-rope placement's `bosses`
record to `Untouched` on `RoomReplayAdmitted`. Every other `BossSpawn` defeat
survives a replay (the save test's own answer list). ⚠ The cut-rope reset reads
every live `BossConfig`, not the replay's live room
(`RoomReplayAdmitted` names the room), so with two live rooms a replay in one
retracts a cut-rope fight in the other.

**Acceptance:** one generic retraction on `RoomReplayAdmitted`, keyed by the
replay's live room, for every boss family whose defeat fell after the replay's
baseline, with the cut-rope special case deleted into it; the reward its
defeat minted goes with it (Q51); a witness per family shape (a conducted boss,
a `BossSpawn` placement) and a control (a defeat from before the baseline
survives the replay).

### MENU-OVER-DIALOGUE — an overlay opened during a conversation must not end it

**Owner:** `crates/ambition_dialog/src/systems.rs` (dialogue input) jointly
with the menu input owner (`crates/ambition_input/src/menu.rs`,
`MenuControlFrame`).

**Ruling (Q75, 2026-10-01, [`maintainer-decisions.md`](maintainer-decisions.md)):**
pause, map and inventory may open during dialogue. The dialogue stays live
underneath without navigation input. Map and inventory are mutually exclusive
primary overlays.

**Current state, measured 2026-10-01.** `apply_dialog_menu_input` closes the
conversation on `menu.back || menu.start`, so the Start press that opens the
pause menu also ends the dialogue. That violates the ruling. The readers of
`MenuControlFrame` (`dialog_input`, the map's `input.rs`, the grid and
kaleidoscope menus) have no ordering or focus between them, so one press can
reach the dialogue and an overlay in the same frame.

**Acceptance:** a Start press during a conversation opens the pause menu and the
conversation is still live, at the same line, when the menu closes; while an
overlay is open, the dialogue reads no navigation; opening the map while the
inventory is open (and the reverse) is refused or swaps, by one stated rule,
with a witness for each.

### CANDIDATE-GENERATION-ORDER — a candidate session is prepared from the generation before its own activation

**Owner:** [`engine/extension-model.md`](engine/extension-model.md) (content
reload) jointly with the session-lifecycle owner —
`crates/ambition_platformer2d_provider/src/lifecycle.rs`.

**Current state, measured 2026-09-19.** `commit_content_generation`
(`game/ambition_content/src/reload.rs`) must run
`.after(AmbitionGameShellSet::Pending)` because that is where
`advance_pending_route` produces `RouteActivated` — reading it earlier was a
measured frame-late bug, fixed 2026-09-12. A10.5 (`c89c68747`, 2026-09-15) then
moved world construction to `prepare_candidate_platformer_session`, which is
`.before(AmbitionGameShellSet::Pending)` so a candidate that cannot be built
never retires the session that is playing — also correct, and the build-here
fallback was deleted, so `adopt_candidate_platformer_session` panics rather
than constructing.

⛔ **BOTH EDGES ARE RIGHT AND THEY PIN OPPOSITE ENDS OF THE SAME SET.** No
ordering edge can put the commit before construction, so a candidate is
necessarily prepared from the content generation GLOBALLY PUBLISHED before its
own activation committed.

⭐⭐ **AND THAT IS NOT THE QUESTION IT LOOKED LIKE, BECAUSE THE ARCHITECTURE
ALREADY ANSWERS IT — `Q147` WITHDRAWN 2026-09-19, THE DAY IT WAS FILED.** The
row read the two edges and concluded a maintainer had to choose between *"the
candidate sees the old generation"* and *"move construction after the
commit"*. There is a third answer already in the tree, and it is the one the
tree is emphatic about: `PendingGenerationInputs`
(`ambition_platformer2d_runtime/src/content_identity.rs`) exists precisely so
a transaction can hand its own N+1 values to its own preparation, keyed by the
`load_id` claim, without global publication — *"THE CANDIDATE MUST NOT HAVE TO
BE PUBLISHED GLOBALLY FOR PREPARATION TO SEE IT… restoring [that] under another
name would undo this whole file."* It already carries the identity line and the
admitted N+1 cast for exactly this reason, and it already names
`fighter_brain_ladder` among the participating families.

⇒ **A stays live on published N; B is built and verified from transaction-local
N+1; adoption makes B/N+1 authoritative atomically.** Neither of the two
options this row posed expresses that, and neither is needed: nothing moves
after the commit and nothing re-fingerprints. ⛔ So what is left is engineering
— bringing any generation input a candidate must see at N+1 into that
transaction-local channel — and NOT a ruling. Filing it as one recreated the
failure mode the 2026-09-19 priority directive names by example.

⚠ **ONE SUCH INPUT IS ALREADY HANDLED ANOTHER WAY AND THE DIFFERENCE IS WORTH
KEEPING.** The fighter ladder is a CONTINUOUS POLICY rather than a construction
input: `project_authored_fighter_ladder` re-reads every fighter and rewrites
only when the rung differs, so a candidate's fighters converge on the published
rung on the tick after the commit without the ladder riding the transaction at
all. A value that must be FROZEN at construction belongs in
`PendingGenerationInputs`; a value that is a standing projection does not.

**2026-10-01: the boss catalog is the second frozen input in that channel.**
Boss profiles and encounters joined the reload; `PendingGenerationInputs::bosses`
carries the admitted N+1 catalog to the transaction's own preparation
(`candidate_bosses_for`), which fingerprints and freezes it, and the commit
publishes it. The poison that dropped it from the claim found a live boss
seeded from the App's catalog after construction, closed the same day
(`BossConfig::seed`). Encounter waves stay a standing projection
(`project_live_encounter_occurrences` reads the App's `EncounterWaveBook` after
activation), like the fighter ladder. Not yet in the channel because not yet
in the reload: none left from the character catalog, which joined the same
day; its admitted cast rides the existing `characters` claim. Measured on the
way: a body's health pool was not read from the frozen cast;
`apply_worn_character_gameplay` re-derived it from the App cast. Fixed
2026-10-01 (`worn_cast_for`); about 30 other live readers of the App cast
remain (see I3).

⚠ **AND THE GUARD STOPPED WITNESSING IT WITHOUT GOING RED.**
`the_commit_sits_between_the_activation_and_session_adoption`
(`game/ambition_app/tests/reload_publication_is_installed.rs`) asserts
`commit → GameplaySessionSet::Providers`. Its own failure message named the
escape hatch — *"if it is not, the provider stopped constructing sessions there
and this ordering names nothing"* — and that is exactly what happened: the set
still exists, so the guard is satisfied, while the construction it was ordering
against left the set. ⭐ Its predecessor was vacuous because ONE SYSTEM DID TWO
JOBS; this one became vacuous because THE JOB MOVED OUT FROM UNDER IT. Both
look identical from the assertion: green. The test now records this and its
name was corrected to what it checks.

**Blocked by:** nothing.

⛔⛤ **THE FIELD WAS MISSING, THEN IT NAMED A QUESTION THAT SHOULD NEVER HAVE
BEEN ASKED.** Adding `Blocked by:` was right — a row that states its gate only
in prose is a gate no derivation reads. Filing `Q147` to fill it was not: the
row's prose said *"it is a maintainer call"* and the field was made to agree
with the prose instead of with the source, where `PendingGenerationInputs`
already answers the question. ⇒ Check whether the architecture has decided
before writing a gate that says nobody has.

**Acceptance:** every generation input a candidate must see at N+1 reaches it
through the transaction-local channel rather than through global publication,
and a value that is a standing projection is stated as one rather than frozen.
⛔ Not by an ordering edge and not by re-fingerprinting: both were refuted
above, and A10.5's "never retire an unbuildable session" guarantee is not
reopened. ✅ **THE GUARD HALF IS DONE — 2026-09-19.**
`the_candidate_is_built_before_the_router_advances_and_providers_only_adopts`
(`crates/ambition_platformer2d_provider/src/lifecycle.rs`) pins the location
rather than the set's existence: construction `.before(Pending)` and
`.after(PlatformerPreparationSet)`, `Providers` holding adoption and NOT
construction. It lives in the crate that owns both systems, because they are
private there and nameable by TYPE — which is what the arm one crate up could
not do, and why it checked a set instead. ⭐ Poison-verified four ways, and two
of the four first failed through `set_key`'s `expect` rather than their own
assertion: **deleting the edge an assertion is about deletes the SET**, so an
absent set now fails the same claim in the same sentence and prints
`set present: false`.

**Diagnosis:** [`triage/a-prose-path-inside-a-doc-comment-is-not-checked.md`](triage/a-prose-path-inside-a-doc-comment-is-not-checked.md)
— found while repairing the comment at `reload.rs` that still named
`activate_prepared_platformer_sessions` and claimed its builder reads <!-- cite-ok: names the system A10.5 deleted 2026-09-15; the row is about that deletion outliving its prose -->
`PreparedCharacterRegistry`; the builder holds no such field, the cast arrives
as a `PreparedContent` argument.

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

**Procedural tier (I4), 2026-10-01:** the first cut landed — an SDK, a Bevy host
and one real technique (the echo fan) running as a module in the shipped game.
State and open parts: [I4](engine/fast-iteration-implementation.md#i4---small-procedural-sdk-and-one-native-semantic-reference).
The same day, the loaded road: the module crate builds as a `.wasm` file in
1.36 s and the same game binary runs it (`AMBITION_EXTENSION_MODULES`), and a
rebuilt file is hot-reloaded through the mechanical-edit protocol. Later the
same day, every boss special became a module (eleven; the native systems are
test-only references held to the modules on the linked and WASM roads), with
three more ports (the boss's live room on the trigger, a boss summon, and a HELD
damage box whose entity the combat domain owns), and D6's first cut (the
declared modules are a section of the prepared content identity). Then the
wielded kit's four damage/projectile items (shockwave, beam, volley, meteor)
became modules on a held-item use port, `record!` gave modules typed records,
and `extension_inspect` shows what runs. Then module-owned entities: a module
asks the world for an entity of a kind it names, and the world ticks it through
the module until its lifetime ends (the sentry is the first, schema 286), and
I5's session-attached records (one record for the session, on its root; a
module the game does not link declares one and it rewinds under GGRS). Next:
save eligibility; the remaining wielded items (dive, blink, grapple,
mark/recall) need ports for body motion (the vortex is a module since, on a
pull port and an end-of-entity port). The Flying Spaghetti Monster's
conductor is a module (a CONDUCTED boss: the module holds the boss's pose,
swings riding hitboxes, rolls held shocks, draws its row; schema 289);
GNU-ton's conductor is the next of its kind.

**2026-10-01, the running game plays content edits.** A build that reads its
content off disk watches the pack's sources (`ambition_content::content_watch`)
and asks for the reload on a save: `request_reload` had no production caller
until then. A saved move retime is played 19 frames after the save; boss
profiles and encounters joined the participating families (a saved boss tuning,
22 frames). A module reload re-mints the session's content identity (D6), and
the module reload's three review findings are closed (a file is the unit of
replacement, one poll is one candidate, a departed schema takes its records).
Later the same day the character catalog joined (a whole-cast revision against
the candidate catalog; a change to which characters are built is refused).
Still refused by the reload: items, audio, fighter facets, the boss seed
library and validator bands. Open, found on the way: a boss's HP, phase
triggers, death seconds, music and reward seed from the App catalog, not the
frozen generation (see I3 in the implementation page).

**Blocked by:** nothing.

⭐ **RULED 2026-09-19.** (Q110) Mechanical registry
changes use proper explicit lifecycle/replacement semantics; do not invent a
universal silent overwrite. (Q104) Content-authored movesets are the long-term
authority and duplicate Rust move tables are MIGRATION SCAFFOLDING, not
permanent architecture — which is this page's own one-fact-one-owner campaign
answering itself. See [`maintainer-decisions.md`](maintainer-decisions.md).

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

**Blocked by:** nothing.

⭐⭐ **RULED 2026-09-19, ALL FOUR AT ONCE, by the
composition-modes decision** (Q100, Q106, Q108, Q97 — with Q146 and Q144).
Capabilities are OPTIONAL and COMPOSABLE; a capability that authored PRODUCTION
content requires and the composition lacks must REFUSE that content or its
admission rather than silently pretending it works, while deliberately reduced
tools and tests may omit capabilities explicitly. ⇒ Q97 is answered REFUSE, and
the profile floor follows from the two supported modes rather than from a census
of hypothetical variants. ⛔ The ruling's own instruction: *"Implement this
architecture rather than continuing to census hypothetical composition
variants."* See [`maintainer-decisions.md`](maintainer-decisions.md).

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

⭐⭐ **THE LOCK IS DIAGNOSED AND HALF-CLOSED. IT IS NOT A SEED BASIN — IT IS
THE ADMISSION RULE, IN TWO PLACES.** Measured 2026-09-19.

**(a) A shove was counted as a reach — FIXED.** `MoveFrameData::coverage` and
`reach` folded over every Active volume, windboxes included, and
[`authoring::wake`] *asserts* that the push reaches further than the hit — so
for every waked move the brain's only statement about where it could land was
the DUST's extent. The goblin's `dirt_kick` read as an 82px poke whose boot stops at
48, and its own shove held the gap open at exactly the range where it could not
hit. ⇒ `MoveFrameData::push_coverage` now carries the windbox region and
`coverage`/`reach` carry the hittable one. Pinned by
`goblin_moveset::dirt_tests::the_brains_reach_for_the_dirt_kick_is_the_boot_and_not_the_dust`
(48 vs 82) and
`options::tests::a_waked_kick_is_priced_by_its_boot_and_not_by_its_dust`.

⭐ **AND THE OTHER FOOT OF THE SPLIT IS CLOSED TOO — 2026-09-19.** Separating
the two regions left a pure shove with no virtue at all: `coverage: None` zeroes
`reach_fit`, `damage: 0` zeroes `expected_payoff`, so the gust was admitted
exactly where it can push and then priced as though pushing were worth nothing
— a move the CPU may pick and has no reason to. The Officer's neutral special
`the_order_to_disperse` is exactly that move. ⇒ `Features::displacement_value`
is the answer: `coverage_fit(push_coverage) × how close the foe is to a blast
line × whether the push sends them THAT WAY`, with its own authored weight.
⛔ NOT `reach_fit` under another name — feeding push coverage back into reach
would re-merge the two regions the split exists to keep apart and price a gust
as a hit.

⛔⛤ **THE THIRD TERM WAS MISSING AND THE FEATURE PAID FOR A RESCUE.** Coverage
says the push REACHES them; proximity says they are near going off; neither
says the push sends them toward the line. Wind blows ONE WAY — the gust's
`push_dir` is authored precisely so it does not flip to suit the geometry — so
an Officer who has crossed to the OUTBOARD side of a cornered opponent shoves
them back to centre with the same coverage and the same proximity, and scored
it as ledge control. `MoveFrameData::push_dir` carries the authored direction
and `outward_local_x` carries which way is off the stage from where the foe
stands, both in the body-local frame `foe_local` already uses. ⚠ Horizontal
only: a side blast line is what a shove threatens, and the vertical component
is a different question. Held by
`a_shove_that_pushes_the_cornered_foe_back_inboard_is_not_ledge_control`, whose
inboard CONTROL is the same foe at the same edge proximity — without it the arm
is satisfied by a gust that never wins anywhere. Poisoned by forcing the term
to 1.0, which reddens it alone.

⛔⛤ **AND THE FEATURE SHIPPED INERT, WHICH THE PIN COULD NOT SEE.** The first
pin, `a_shove_outranks_a_near_miss_at_the_ledge_and_not_at_centre`, scored a
synthetic 60px gust against a synthetic 40px jab: the gust won beside the blast
line (0.85 to 0.81) and lost at centre (0.38), and a weight of `1.0` was fitted
to that. Measured 2026-09-19 against the kit `attack_kit_of` actually builds
from the Officer's table — eleven candidates — **the gust placed FOURTH beside
the ledge** and never won anywhere. Its rival is not a jab: it is
`officer_tilt_forward`, 7 damage, faster, reaching 48px. ⇒ **A two-move fixture
cannot price a move against a kit**, and a feature is not delivered when its
unit test passes.

⭐ **RE-MEASURED AND LANDED.** Sweeping the weight against the real kit at seven
positions gives a band of `1.25..2.6`: below 1.25 the gust never wins, above 2.6
it wins at centre stage. `UtilityWeights::v1().displacement_value` is now
**1.8**, the middle of it. Pinned by
`officer_moveset::he_uses_the_gust::he_shoves_at_the_ledge_and_punches_at_centre`
— the gust is the best attack at both blast lines and at neither centre-stage
gap — with
`the_gust_is_on_the_table_at_centre_stage_and_simply_loses` as the anti-vacuity
half. Poisoned at 1.0 (both ledge arms red) and at 3.0 (the centre arm red),
each through its own assertion.

⚠ **POPULATION: TWO MOVES IN THE WHOLE ROSTER**, pinned by
`exactly_two_moves_in_the_roster_author_a_push_region`. `officer_disperse` is
the pure shove; the goblin's `dirt_kick` also hits, so it competes on
`reach_fit` like an ordinary move and never wins at any weight up to 3.0. ⇒ The
entire behavioural delta of this number is ONE move on ONE character, and the
duel stays bit-identical at rungs 6 and 9 for the same reason it did before:
the pirate admiral's kit carries no windbox.

**(b) "An attack that cannot reach is not an option" admitted anything within
THREE TIMES its reach — FIXED 2026-09-20, and the fix needed three more
owners of "how far does this move reach" before it could land.** The filter
asked `reach_fit > 0.0`, and `REACH_TOLERANCE` is `2.0`, so the soft score
stayed positive until the gap was `3 × reach`. A 48px kick was an option at
140px; `attacks.first()` always answers; and a committed move owns the body for
longer than the gap between decisions. ⇒ The fighter pressed an unreachable
move, could not walk while it played, and the next free tick found the same
world.

⚠ **THE TOLERANCE IS RIGHT FOR A RANKING AND WRONG FOR AN ADMISSION**, which
is why the repair is a second function rather than a smaller constant: a
near-miss SHOULD rank near a hit — that is what makes a brain commit to a
spacing — but a near-miss is still a miss, and a menu that offers one has no way
left to say *"walk in first"*.

⭐⭐ **BUILT AND MEASURED ON THE WHOLE GRID, 2026-09-20.** Admission is now
absolute — the opponent must lie between the near and far sides of the move's
own region, forgiving `ADMISSION_SLACK_PX` (24px) on each — while scoring keeps
the `REACH_TOLERANCE` falloff, which is the split this row asked for.

⚠ **THE CONTROL IS ONE RUN OF THE SAME INSTRUMENT, NOT A REMEMBERED NUMBER.**
Every column is `every_fighter_on_the_grid_can_fight_its_mirror`, 21 mirror
matches of 3600 ticks, the same binary, the control taken with only the
behavioural changes stashed. The middle column is kept because it is the
evidence for the last paragraph of this section. The number after each pair is
DISTINCT moves started, which is the lock this row is named for.

Total accumulated damage **5019% → 6350% → 6918%**, and every one of the 21
rows moved.

| fighter | control @ `5e68f6052` | admission only | + leading the aim | Δ |
|---|---|---|---|---|
| `npc_bob` | 55% / 71%, 9 | 205% / 193%, 11 | **234% / 247%, 16** | +355 |
| `npc_pirate_admiral` | 48% / 82%, 15 | 70% / 102%, 13 | **233% / 235%, 13** | +338 |
| `mary_o_tall` | 80% / 101%, 15 | 237% / 206%, 14 | **206% / 244%, 17** | +269 |
| `goblin` | 12% / 12%, **1** | 164% / 169%, 13 | **144% / 119%, 17** | +239 |
| `npc_alice` | 76% / 73%, 10 | 143% / 129%, 20 | **178% / 173%, 20** | +202 |
| `smash_george_booul` | 122% / 187%, 14 | 372% / 330%, 16 | **252% / 202%, 17** | +145 |
| `pugnacious_polygon` | 129% / 156%, 20 | 113% / 115%, 13 | **218% / 198%, 20** | +131 |
| `perfect_cellular_automaton` | 151% / 138%, 18 | 128% / 124%, 15 | **210% / 195%, 18** | +116 |
| `projectile_polygon` | 132% / 162%, 17 | 187% / 151%, 17 | **179% / 229%, 17** | +114 |
| `npc_emmy_noether` | 73% / 110%, 12 | 265% / 265%, 9 | **146% / 146%, 11** | +109 |
| `player_robot_v3` | 179% / 204%, 19 | 142% / 155%, 18 | **225% / 223%, 19** | +65 |
| `npc_ninja_shadow_oni_leader` | 125% / 178%, 15 | 251% / 221%, 11 | **179% / 182%, 8** | +58 |
| `pointed_polygon` | 157% / 236%, 18 | **0% / 0%, 3** | **199% / 218%, 16** | +24 |
| `special_patent_clerk` | 0% / 0%, **1** | 221% / 234%, 17 | 11% / 11%, 2 | +22 |
| `medic` | 151% / 158%, 21 | **0% / 0%, 1** | **155% / 155%, 1** | +1 |
| `sanic` | 46% / 54%, 12 | **0% / 0%, 7** | 47% / 52%, 4 | −1 |
| `director` | 119% / 145%, 18 | 85% / 99%, 17 | 109% / 142%, 19 | −13 |
| `npc_carl_stargan` | 103% / 141%, 14 | 175% / 138%, 15 | 102% / 94%, 10 | −48 |
| `officer` | 81% / 143%, 17 | 67% / 70%, 13 | 78% / 97%, 17 | −49 |
| `performer` | 146% / 163%, 17 | 193% / 174%, 17 | 97% / 133%, 14 | −79 |
| `npc_oiler` | 230% / 290%, 18 | 217% / 240%, 16 | 189% / 232%, 17 | −99 |

⛔⛤ **AND THE MIDDLE COLUMN IS WHY THERE IS A THIRD ONE. TIGHT ADMISSION ASKED
ITS QUESTION OF A WORLD THAT HAD ALREADY MOVED ON.** Three fighters fell to
**0%** there, and the reading that found it contradicts the rule's own
ceiling: `medic_tourniquet` reaches 80px, so it is admitted only out to 104px,
and `AMBITION_GRID_TRACE` caught a `medic` mirror STARTING it at a real gap of
**153.6px**, 177 times in 3600 ticks, with `LandedBodyHit` at zero for the
bout. A move cannot be admitted past its own ceiling. What was admitted was a
REMEMBERED opponent.

⇒ `DelayedPerception` is the no-cheat contract made structural, and the delay
was invisible to everything downstream: a consumer got a `WorldView` and no way
to ask how late it was. At rung 5 that is `reaction_ms: 300` — eighteen ticks —
plus the move's own startup still to come, about 100px of walking against an
`ADMISSION_SLACK_PX` of 24. It went unnoticed for as long as admission forgave
three times a move's reach; **making the rule honest is what made the staleness
matter**, which is why (b) could not land alone.

⭐ **`Perceived::staleness_s()` is the repair, and it is one field on the view
rather than a parameter on a seam.** The buffer reads it off the views' own
`sim_time` — no tick rate to agree on, and warm-up (which deliberately returns
the OLDEST view held, not one `delay_ticks` old) reports itself correctly
instead of being overstated from the configuration. Attack admission then
carries the foe forward at the relative velocity the view reports, over
`staleness + startup`.

⚠ **SCORING IS DELIBERATELY LEFT ON THE OBSERVED POSITION**, and this is the
line that keeps the difficulty ladder intact: `reach_fit` is a judgement about
VALUE, `reaction_ms` is the shipped difficulty axis, and a brain that predicted
perfectly everywhere would flatten it. Admission is the ONE judgement here
about a moment in the future — the tick the hitbox opens — so it is the one
that leads. The 24px constant was a stand-in for exactly this and stays for the
residue (the decision interval, which the option layer does not know).

⭐ **IT ALSO CLEANED UP THE LADDER RIG**, which is a second, independent
witness. `the_ladder_is_ordered_by_press_rate` used to dip at rungs 7 and 9 and
needed a two-rung window to survive; the curve is now strictly increasing at
every adjacent pair up to rung 7 — `29.3 33.3 37.3 41.3 44.7 48.7 52.7` — and
flat at ~52 above it, because uncapped this rig presses ~55 however hard the
rung is. Higher rungs see a fresher world and therefore lead LESS, and the
delay had been costing them presses the cap was supposed to be the only thing
withholding. The test's claim is now strict monotonicity while the cap binds
plus no fall-back above it, with `CAP_BINDS_THROUGH` derived from the `no cap`
null control rather than chosen.

⚠ **STILL BELOW CONTROL, AND NOT CHASED:** `npc_oiler` (−99 of 520, 19%),
`performer` (−79), `officer` (−49), `npc_carl_stargan` (−48). None is a
collapse and each keeps 10–17 distinct moves. `special_patent_clerk` is the odd
row — 0% on ONE move in control, 455% on seventeen with admission alone, 22% on
two with the lead — and he is the next thread to pull, because whatever the
lead does to him it is not what it does to anybody else.

⭐ **AND THE MIRROR ITSELF IS A POPULATION, WHICH THE ZEROES TAUGHT.** Two
copies of one brain at one rung rank the same menu the same way; once it
narrows to one or two moves the pair locks into a deterministic limit cycle and
repeats a period exactly, so the move either lands every time or never. Seat
somebody else opposite and all three of those fighters fought:
`AMBITION_GRID_FOE=smash_george_booul`, same clock — `medic` **80% on 23
distinct moves**, `pointed_polygon` **88% on 24**, `sanic` **17% on 13**. ⇒ A
`0%` row in a mirror is a QUESTION, not a verdict; the sweep's assertion now
says so in its own message, and `AMBITION_GRID_ONLY` / `AMBITION_GRID_FOE` /
`AMBITION_GRID_TRACE` are how it gets answered.

⚠ **AND ONE SHIPPED FIXTURE WENT RED FOR A REASON THAT WAS NOT A REGRESSION,
WHICH ITS OWN COMMENT HAD PREDICTED ABOUT ITSELF.**
`the_repertoire_gets_used::the_cpu_charges_a_smash_and_techs_a_landing_in_some_match`
ran THREE noise streams and asked for one charge, above a paragraph reading
*"it passed for a week and then failed on a change that made the fight
BETTER"*. It did that again. Measured over TEN streams instead of three:
charges in **3** (0.98, 0.99, 1.00), so the per-stream rate is about 0.3 and
three draws decide the test only two times in three. The same runs lifted techs
from a recorded 0–29–54 to 44–116 and tumbles from 0–97–121 to 21–407 — the
fight was livelier and the sample was the same size. `STREAMS` is now set from
that rate, and the failure message says what the sample can decide.

⛔⛤ **AND CHASING ONE OF THOSE ROWS FOUND A SECOND OWNER OF "HOW FAR DOES
THIS MOVE REACH".** A capture rides an Active window's `sustain_effect`, not
its volume list, so `MoveSpec::frame_data` — which folds over `volumes` — said
every grab on the roster has NO region at all, and an option scorer reads
`coverage: None` as *"this move cannot miss"*. `capture_candidate` in the actor
layer patched the coverage, the reach and the unblockability back in, for
exactly ONE move: the neutral grab it reaches through `GRAB_VERB`. ⇒ A command
grab bound to an ordinary attack verb carries the same `CAPTURE_ATTEMPT` params
and got none of it. `pugnacious_polygon/polygon_brawler_collar` reaches 58px on
`attack_side` and read as reachless, which is what he threw 38 times in 66
starts at a mean gap of 143px. The derivation is `frame_data`'s now, where the
key and the params are declared, and the actor-layer copy is deleted. Held by
`both_of_his_grabs_tell_the_brain_the_distance_they_close`, whose CONTROL is
his standing grab — the one that was already right.

⛔⛤ **AND A THIRD OWNER: A MOVE CAN REACH THROUGH SOMETHING IT IS NOT.** With
grabs fixed, the fighters that still fell were throwing counters and buffs,
because `coverage: None` was the same answer for four unrelated things.
`MoveFrameData::hazard_reach` is the new datum (since replaced by `MoveFrameData::hazard: Option<MoveHazard>`, `ambition_entity_catalog/src/hazard.rs`), derived in `frame_data` <!-- cite-ok: the bare distance this paragraph introduced; its successor is named beside it -->
beside the capture fold, and it has **two roads, not one**:

1. An EFFECT KEY the catalog has been taught. `hazard_reach_of` prices
   `smash_bolt::STEERED_BOLT` at `offset + speed × lifetime + radius` and
   `smash_bomb::DROP_BOMB` at `offset + blast_radius`. ⛔ It names the keys it
   has NOT been taught (`PLACE_MINE`, `MARK_BODY`, `TETHER_PULL`,
   `HOMING_DASH`) and states that an untaught key's zero is a REFUSAL.
2. ⛔⛤ **THE OWNER'S OWN TRIGGER, WHICH IS NOT A KEY AT ALL AND NEARLY TOOK A
   WHOLE FIGHTER OFF THE MENU.** An ordinary ranged move fires the BODY's
   `RangedActionSpec` through `MoveEventKind::Ranged`, and both of
   `projectile_polygon`'s neutral options author no Active volume — *"the
   projectile IS the damage, as it is for every ranged move"*. A rule built
   from the effect-key table alone would have deleted her entire game. They
   answer `RANGED_ACTION_REACH`, a stage-crossing placeholder, because the
   BODY owns the shot's real speed and flight and a catalog derivation has no
   body. ⇒ **UPDATED 2026-09-20:** the kit builder does narrow it now, and the
   datum is no longer a bare distance — `MoveFrameData::hazard` is an
   `Option<MoveHazard>` whose `OwnersRangedAction` variant is a REQUEST this
   placeholder only stands in for. See the resolved-action-offer block below.
   Held by `her_two_neutral_projectiles_tell_the_brain_they_cross_the_stage`,
   whose CONTROL is her bomb — which must answer its own arc rather than the
   placeholder, or the test passes against a catalog that answers every
   hitless move 1000px.

⚠ **AND THAT CONTROL EARNED ITS KEEP IMMEDIATELY.** The first `DROP_BOMB`
arithmetic was `impact_speed × fuse_s + blast_radius`, which put her bomb at
1096px — further than the bolt and further than the stage. `impact_speed` is a
DETONATION THRESHOLD (*"minimum contact speed that detonates the bomb"*), not a
launch speed; a drop bomb is DROPPED, so its reach is where it lands plus the
blast. Read the field's own doc before multiplying it by a time.

⛔⛤ **AND ADMITTING THE RANGED MOVE EXPOSED THAT IT WAS BEING AIMED BY THE
WRONG CLOCK.** The lead carries the foe forward over `staleness + startup_s`,
and `startup_s` is *"time until the first Active window"* — which **falls back
to the WHOLE MOVE DURATION** when there is none. That is the shape of every
ranged move in the game, so the moment they reached the menu they were aimed
past the opponent:

| move | throws at | `startup_s` | error at 200px/s |
|---|---|---|---|
| `polygon_projectile_charge_shot` | 0.26s | 0.58s | 64px |
| `polygon_ponytail_boomerang` | 0.16s | 0.40s | 48px |
| `polygon_lay_bomb` | 0.18s | 0.46s | 56px |

Each is wider than the `ADMISSION_SLACK_PX` this rule is tuned around. ⇒
`MoveFrameData::threat_live_at_s` is **when this move first offers the opponent
anything**, folded from the same three roads the hazard fold uses — an Active
window's own `start_s`, a `Ranged` or hazardous `Effect` event's `at_s`, and a
hazardous sustain's window — and `None` when it offers nothing, which is
exactly the population the attack menu's third arm refuses. ⛔ NOT an overload
of `startup_s`: the two are equal for an ordinary strike by construction and
separate only where the move reaches through something it spawns. Held by
`a_projectile_threatens_when_it_is_thrown_not_when_the_move_ends` against the
authored constants, with `a_strike_threatens_when_its_hitbox_opens` as the
control — without which *"the threat time is early"* is also true of a field
that is simply always early.

⭐⭐ **THE WHOLE 2026-09-20 REVIEW BATCH, JUDGED ON THE GAME: 6918% → 6775%,
AND EXACTLY THREE OF TWENTY-ONE ROWS MOVED.** Same instrument, same 21 mirror
matches of 3600 ticks, same binary; the column it is measured against is
`+ leading the aim` in the table above. The three corrections in the batch are
the shark off `hazard_reach`, `threat_live_at_s` replacing `startup_s` in the
lead, and a travel-aware `MotionOption`.

| fighter | + leading the aim | + the review batch | Δ | why this row and not the others |
|---|---|---|---|---|
| `npc_pirate_admiral` | 233% / 235% | 239% / 201% | −28 | `call_the_shark` leaves the attack menu. It is the roster's only `SustainedAuthority`. |
| `projectile_polygon` | 179% / 229% | 184% / 229% | +5 | Her boomerang, charge shot and bomb are aimed by their throw time instead of the end of the move. |
| `director` | 109% / 142% | 34% / 97% | −120 | His steered bolt is the roster's only `STEERED_BOLT`. ⭐ **AND THIS ONE IS THE MIRROR, NOT THE CHANGE** — see below. |
| the other 18 | — | — | **0** | bit-identical, every one. |

⭐ **THE `director` ROW IS A QUESTION AND IT HAS BEEN ANSWERED.** A 34%/97%
split inside a MIRROR is the asymmetry this row's own protocol says to re-seat:
`AMBITION_GRID_FOE=smash_george_booul`, same clock, gives **218% / 131% on 30
distinct moves** with the bolt thrown 19 times at a mean gap of 57px — against
11 distinct and 39 throws at 158px in the mirror. He is not broken; two copies
of one brain narrowed onto one move again.

⚠ **AND THE BOLT DOES POINT AT REAL UNFINISHED WORK, WHICH IS WHY THE ROW WAS
WORTH CHASING.** `threat_live_at_s` is when the bolt is THROWN, and a bolt is
not a threat where it is thrown — it crosses 671px at 300px/s, so it arrives up
to two seconds later. Replacing an over-lead (the whole move duration) with an
under-lead (zero flight time) is an improvement and not the answer: the truthful
number is `throw + gap / hazard speed`, and the hazard's SPEED is exactly what
`MoveFrameData` does not carry — `hazard_reach_of` folds it into a distance and
throws it away. Which is the next slice, below.

✅ **THE BRAIN AND THE HIT RESOLVER NOW SPEND ONE LAUNCH LAW — 2026-09-20,
review finding #2.** There were two copies. `ambition_combat` resolved a hit as
`base + growth × growth_base(base) × growth_scale × victim_damage / weight`,
all folded with rage; `LaunchEnvelope::at`, which is what ranks a fighter's
finishers, evaluated `base + growth × victim_damage` and its own doc argued the
rest away — the omitted factors are *"COMMON to every candidate one attacker
weighs against one opponent, so none of them can reorder a kit"*.

⛔ **THE ARGUMENT IS WRONG AND IT IS WRONG WHERE IT MATTERS.** Every omitted
factor multiplies the PERCENT TERM and not `base`, so it moves the CROSSOVER
between two candidates rather than scaling both lines alike:

```text
d* = (b₂ − b₁) · weight / (growth_scale · growth_base · (g₁ − g₂))
```

George Booul's forward smash is `(185, 3.45)` and his up smash `(178, 6.28)`.
At **two** points of victim damage the brain preferred forward (191.90 against
190.56); on the smash stage against a Robot v2 the same two moves resolve to
195.15 against 196.47 — up smash has already overtaken. TWO factors do it:
the stage's declared percent scale `1.25` and the victim's authored weight
`0.85`. No curve is needed, which matters because the stage no longer declares
one (below).

⇒ **ONE OWNER: `ambition_entity_catalog::launch`.** `launch_speed(base,
growth, LaunchConditions)` is the only copy — set knockback short-circuits to
`base` and declines rage, and `GrowthBaseCurve` moved here with it.
`ambition_combat::util::scaled_knockback` and `rage_for_growth` are DELETED;
`ambition_combat` still resolves every input (`ResolvedCombatTuning`) and no
longer owns the arithmetic they are spent on, which is what lets the fighter
brain — which cannot see `ambition_combat` — spend the same law the hit
resolver does. The hitbox road, the throw road and `LaunchEnvelope::at` all
call it.

⭐ **AND THE STAGE'S LAW REACHES THE BRAIN THROUGH THE VIEW, WHICH IS WHERE
PRIVILEGE WOULD HAVE BEEN EASY.** `WorldView::launch_law` carries
`growth_scale`, `growth_base` and this body's own `rage`;
`PerceivedActor::knockback_weight` carries the foe's. A ruleset's percent curve
is a property of the stage a fighter is standing on and a body knows how hurt
it is — the no-cheat rule is about the OPPONENT's hidden state, and this is
neither hidden nor the opponent's. One factor is deliberately absent and named
rather than argued away: per-move STALING, which the runtime folds into
`growth_scale` and a brain with no usage history cannot. That one genuinely can
reorder a kit; it is the whiff/usage-memory slice below.

⛔⛤ **THE PLUMBING IS WITNESSED IN THE COMPOSED HOST, BECAUSE EVERY LAYER OF IT
WOULD PASS WITH NOTHING INSTALLING THE RESOURCE.** Each combat reader carries
`Option<Res<ResolvedCombatTuning>>` for headless minimalism, so "absent"
resolves to the identity law and the brain silently returns to the old ranking.
`smash_in_the_host::a_seated_fighters_view_carries_the_launch_law_this_stage_declares`
boots the shipped host, seats George against a CPU, and reads the CPU's own
delayed view: `growth_scale == 1.25` against an engine default of `1.0`, and a
perceived `knockback_weight` of `1.35` against an unauthored body's `1.0`. Both
poisoned separately and both reddened through their own assertion.

⚠ **AND THE STAGE DECLARES NO GROWTH-BASE CURVE, WHICH TWO NEW TESTS HAD TO BE
CORRECTED FOR BEFORE THEY LANDED.** `ambition_demo_smash` ran
`GrowthBaseCurve { pivot: 48, exponent: 0.25, ceiling: 1.40 }` and RETIRED it
(`growth_base: None`, with its reasons beside it: base knockback is not a
move's role, an authored `knockback_growth` stopped meaning what it says, and
it never reached throws at all). Both arms had been written citing the retired
constants as *"the law the stage declares"* — a test certifying a model nothing
runs, which is the same failure the shark arm was reverted for the day before.
`SMASH_GROWTH_BASE_{PIVOT,EXPONENT,CEILING}` are still in the demo and are now
documentation of a retired law.

⭐⭐ **JUDGED ON THE GAME: 6775% → 7059% (+4.2%), THREE OF TWENTY-ONE ROWS
MOVED AND ALL THREE MOVED UP.** Same instrument as the batch above — 21 mirror
matches of 3600 ticks — against the sweep taken at `570f81c0c`.

| fighter | before | after | moves | distinct | most thrown |
|---|---|---|---|---|---|
| `npc_bob` | 234% / 247% | 270% / 281% | 103 → 107 | 16 → 18 | `piston_charge` ×19 → ×20 |
| `npc_oiler` | 189% / 232% | 224% / 263% | 103 → 108 | 17 → 17 | `slick_dash` ×16 → ×17 |
| `npc_carl_stargan` | 102% / 94% | 161% / 183% | **24 → 62** | **10 → 18** | `planetary_orbit` ×7 → ×15 |
| the other 18 | — | — | — | — | bit-identical |

⚠ **THE SHAPE IS THE SAME IN ALL THREE AND IT IS THE SHAPE THIS CHANGE
PREDICTS:** more of the kit used, more presses, more hitstun. `kill_potential`
divides by `kit_max_launch`, so a percent scale common to the whole kit
CANCELS; what moves is the ORDER, and a fighter whose second-best finisher was
being offered first now has a different move at the top of its menu. Carl is
the loud one — 24 move starts to 62 on 10 distinct to 18 — and he is the row
most exposed to it, a kit of big-base pulses whose crossovers all sit low.
⛔ No row fell, which is the reading that would have sent this back.

⚠ **ONE DIVERGENCE IS STATED RATHER THAN HIDDEN, AND IT IS LATENT.** The throw
road passes `GrowthBaseCurve::IDENTITY`: it never applied the curve, and before
the shared law the omission looked like an absence of code rather than a
decision. A declared curve would steepen a big-`base` VOLUME and leave a
big-`base` THROW alone. Nothing declares one today, so every world resolves it
to identity anyway and the constant keeps today's numbers to the byte — what it
costs is a trap for whoever declares the next curve, which is why the question
now sits in the signature.

⛔⍤ **AND ONE SHARED FUNCTION WAS NOT ENOUGH — THE SAME REVIEW CAME BACK THE
NEXT DAY AND FOUND THE TWO SIDES STILL DISAGREEING ABOUT `knockback_growth:
None`.** `knockback_growth` has TWO authoring roads and they mean different
things. `Some(g)` states a growth, and `Some(0.0)` is the documented way to
author a FIXED launch — a windbox that throws the same distance at 0% and at
200%. `None` says *"the ruleset decides"*, which the hit resolver answers with
`base × DeclaredCombatRules::knockback_growth`. Moving the arithmetic into one
function did not make the sides agree, because each still COLLAPSED the
`Option` before calling it, and they collapsed it differently: the resolver
spent the fallback, `LaunchEnvelope::with_volume` spent `unwrap_or(0.0)`.

⛔ **LIVE ON THE SHIPPED ROSTER, AND NOT SMALL.** Exactly two moves in the
game author `None` — `cellular_pulse` at base 140 and `performer_trapdoor` at
150, measured over every `movesets/*.ron`, and no move MIXES the two roads,
which is what keeps the envelope's two lines exact. The smash stage declares
`knockback_growth: 0.02` with a percent scale of `1.25`, so at 100% against
the reference body the stage throws the pulse **490px/s** while the brain
priced it **140** — and, reading it as a set launch, declined its rage as
well. The brain called the roster's two ruleset-scaling specials set knockback
and ranked them as pokes.

⇒ **THE COLLAPSE BELONGS TO THE LAW, NOT TO ITS CALLERS.** `launch_speed(base,
growth: Option<f32>, conditions)` takes the `Option` and `LaunchConditions`
carries `ruleset_growth`, so there is exactly one place in the workspace where
a `None` becomes a number and it is inside the thing both sides call.
`LaunchEnvelope`'s flat line keeps the `Option` rather than resolving it at
authoring time, and `grows()` became `grows_under(conditions)` because the
question genuinely has two answers — a `None` volume is a set launch in an
undeclared world and a percent-scaling one on a stage that declares a
fallback.

⭐ **THE WITNESS IS AN EQUALITY BETWEEN TWO CRATES, NOT A PINNED NUMBER**, and
it lives in `ambition_combat` because that is the one crate that can see both
the hit resolver and `LaunchEnvelope`.
`hitbox::tests::the_brain_and_the_resolver_agree_about_a_volume_that_authors_no_growth`
takes the pulse's own `(140, None)` and asks both roads under three worlds:
undeclared (`ruleset_growth` 0) → a set launch at 140; the smash stage
(`0.02`, `1.25`) → 490; and `Some(0.0)` → 140 under BOTH, which is the control
that stops the arm passing for a law that ignores the `Option` entirely. Four
poisons, four different reddenings: collapsing on the brain side only fails the
cross-road equality; dropping the fallback from the shared law moves BOTH sides
together and is caught by the literal 490 instead; ignoring the authored growth
fails the `Some(0.0)` control; and a `grows_under` that does not read the
ruleset fails the last arm on its own message.

⭐⭐ **JUDGED ON THE GAME: 7197% → 7195%, and the interesting number is that
EXACTLY THE TWO PREDICTED ROWS MOVED.** The prediction was written before the
sweep ran: only `perfect_cellular_automaton` and `performer` carry a `None`
volume, so only those two rows may differ, and any third would mean the change
leaked.

| fighter | before | after | hitstun | most thrown |
|---|---|---|---|---|
| `perfect_cellular_automaton` | 210% / 195% | 201% / 195% | 1441 → 1432 | `glider_launch` ×29 → ×28 |
| `performer` | 97% / 133% | **104%** / 133% | 596 → 616 | `performer_the_line` ×21 → ×20 |
| the other 19 | — | — | — | bit-identical |

⚠ **NET −2 OF 7197 IS FLAT, AND THAT IS THE HONEST READING OF A CORRECTNESS
FIX.** The two rows moved in opposite directions. `kill_potential` divides by
`kit_max_launch`, so raising ONE move's launch inside an eighteen-move kit
mostly raises the denominator every other move is measured against — the
ranking shifts a little and the fighter's total output barely does. What the
change buys is not damage: it is that the brain and the stage now agree about
what a move does, so the next thing built on the launch price is built on the
one the game actually pays. The measurement that would have sent this back is a
row moving that could not have.

✅⭐ **THE HAZARD CARRIES ITS OWN LAW NOW, NOT A SPEED — 2026-09-20, the
reorientation the second review asked for:** *"stop extending `MoveHazard`
with additional scalar approximations."* A pair `{reach, speed}` can only
describe uniform motion, and two of the four hazard shapes the roster already
ships are not uniform. Each flattening cost a wrong answer in the same units
as a right one.

⛔ **A BOOMERANG IS NOT A CONSTANT SPEED, AND ITS AVERAGE IS RIGHT AT EXACTLY
ONE DISTANCE.** The first repair published `travelled / travel_s` so that
`reach / speed` came out right — and it does, at the turnaround and nowhere
else, because the shot decelerates the whole way out. Projectile Polygon's
ponytail, `v0` 430px/s turning at 0.34s:

| centre travel | the law | the average model |
|---|---|---|
| 40px | **0.111s** | 0.186s |
| 60px | **0.196s** | 0.279s |
| 73.1px (turnaround) | 0.340s | 0.340s |

At 200px/s of closing speed the first row is 15px of excess lead, which is
`ADMISSION_SLACK_PX` to the pixel.

⛔ **AND A LAID BOMB IS NOT A STATIONARY PROJECTILE.** `DROP_BOMB` published
`speed: 0.0`, documented as *"its whole reach is available the moment it
exists"* — the opposite of what the move authors: *"laying a bomb is not a hit
— the bomb is."* `DropBombParams::fuse_s` is *"seconds until it goes off by
itself"*, **four** of them on the shipped polygon. The test that certified the
old reading used the bomb as its control for *"all reach immediately
available"*.

⇒ **`ThreatTravel`, and the question it answers is `travel_to(distance)`.**

```text
Straight  { speed, span, free }   t = (d - free) / speed
Boomerang { v0, out_s, free }     t = out_s - sqrt(out_s² - 2·out_s·(d-free)/v0)
Placed    { reach, earliest_s }   t = 0        (it is placed where it is placed)
```

`free` is ground the hazard covers without flying — its spawn offset, its own
half-extent, any splash — because a shot touches somebody with its edge. Every
consumer of the old pair was dividing a gap by a speed; a shape that knows its
own law answers directly, and one that CANNOT reach a distance returns `None`
instead of a number. Four variants, all shipped content; a fifth is a new law
and not a new scalar on an existing one.

⛔⛤ **THE FUSE IS NOT A FLIGHT TIME, AND ONE SWEEP WAS SPENT PROVING IT.**
The first version answered both questions with one function, so the bomb's
four seconds were fed to the AIM LEAD — which carries the opponent forward at
the velocity last seen. Four seconds of that is arithmetic about a walk nobody
takes: her bomb reaches **72px** (`offset -16`, `blast_radius 56`), so at any
walking speed at all the extrapolated opponent is outside it. The grid said so
immediately — `projectile_polygon` **144/228 → 131/183**, repertoire 17
distinct moves → 15. That is a move being DELETED, not corrected.

⚠ **AND THE FIRST EXPLANATION WAS WRONG, WHICH IS WHY IT WAS MEASURED BEFORE
IT WAS KEPT.** The reflex was to cap the lead at the width of the room — a
480px stage cannot contain 800px of walk. Against her real numbers it cannot
be the mechanism: 72px of reach is exceeded by 4 seconds at 18px/s, and the
cap only bites above 120px/s. It was reverted unmeasured rather than kept as a
plausible knob. ⇒ The split is on the TYPE: `travel_to` is the aiming question
(*where do I point this so it lands on them*) and `live_at_s` is the fuse
(*when can it hurt anybody at all*). A placed object is aimed nowhere and
travels for zero. **Nothing prices the fuse yet, and that is written down on
the type rather than patched into the lead** — *"will they be within 72px in
four seconds"* is a stage-control question, the same shape as the counters and
buffs already held off the attack ranking until there is a defensive feature
to price them with.

⚠⬤ **TWO DEBTS ON `Placed`, RAISED BY REVIEW OF `11f402eebbc0` AND LEFT
OPEN ON PURPOSE.**

1. **A PLACED TRAP HAS A POSITION, AND `reach` IS A RADIUS.** The polygon's
   bomb is authored at `offset (-16, +14)` with a 56px blast — deliberately
   BEHIND and below her — and `hazard_of` collapses that to
   `|offset.x| + blast_radius` = 72px. Two opponents 70px in front and 70px
   behind get the same admission answer though the bomb is 32px closer to one
   of them, and the `y` is discarded outright. ⇒ The repair is to carry the
   dangerous REGION relative to the body, at the resolved-offer seam, **not**
   another reach scalar. Not done here: 32px of asymmetry on a 480px stage is
   smaller than the observability gap that would tell us whether it matters in
   play.
2. **`detonates_by_s` IS A DEADLINE, AND IT WAS NAMED `earliest_s` FOR ONE
   COMMIT.** The bomb goes off *"in four seconds OR on a sufficiently hard
   impact, whichever happens first"*, and the runtime implements exactly that,
   so four seconds is the LATEST it waits rather than the soonest it can go.
   Renamed here rather than deferred, because nothing consumes it yet and the
   next thing to read it is a diagnostic — a misleading field is worse once
   something reports it as authoritative. The impact road depends on what
   somebody else does to the object and stays unmodelled.

⛔⛤ **AND AN UNRESOLVABLE RANGED REQUEST IS NOW NO OFFER AT ALL.**
`resolve_owners_ranged_action` returned early when neither an equipped weapon
nor the body's standing kit could answer, which LEFT `OwnersRangedAction` in
the frame data — and its unjoined `reach()` is `RANGED_ACTION_REACH`, 1000px,
wider than any stage. So the one layer that had just proven the press fires
nothing handed the brain an instantaneous stage-crossing threat. The previous
arm asserted that ON PURPOSE, reasoning that *"the move fires nothing"* is a
question for whoever decides pressing — and this IS that layer. No shipped
fighter reaches the state, so it is a structurally invalid API state rather
than a reproduced bug. The join clears the hazard, keeps the CANDIDATE (a body
does not lose a move because of what it is not carrying), and
`MoveHazard::travel_to` refuses to answer for the variant so nobody can
quietly re-derive one.

⭐⭐ **JUDGED ON THE GAME: 7195% → 7201%, ONE ROW MOVED.** The two sweeps are
a controlled pair — they differ only in whether the fuse reaches the lead —
and that is what identifies the polygon's loss as the fuse and nothing else.

| fighter | before | after | moves | distinct | most thrown |
|---|---|---|---|---|---|
| `director` | 132% / 129% | 123% / **144%** | 129 → 104 | 17 → 18 | `director_train_of_thought` ×67 → ×45 |
| `projectile_polygon` (fuse in the lead) | 144% / 228% | 131% / 183% | 123 → 114 | **17 → 15** | — |
| `projectile_polygon` (fuse out) | 144% / 228% | 144% / 228% | — | — | bit-identical |
| the other 20 | — | — | — | — | bit-identical |

⚠ The director is the roster's one steered bolt and the only row the flight
law can reach. He throws it **a third less often** and uses one more distinct
move, at a mean gap of 151px against 127px — the bolt's own 10px body and
spawn offset are ground it does not have to fly, so the shot is admitted when
it will land instead of reflexively. His `took1` rose 129% → 144%.

✅ **A BURST COVERS GROUND ONLY AFTER ITS IMPULSE FIRES — 2026-09-20, review
finding #3.** `travel_of` priced a pure-motion move at `speed × total_s`, so a
move whose impulse arrives a fifth of the way in was credited with a fifth
more distance than the body ever travels. `medic_rescue_lift` applies
`(34, -905)` at 0.12s of a 0.48s move: **435px** priced against **326px**
travelled. The horizon is still `total_s`, which is a convention rather than a
measurement — an impulse SETS a velocity and the body keeps it past the end of
the move — but it is the same commitment window every other option on the list
is priced over, so the candidates stay comparable. What is fixed is the part
that counted motion during frames the body provably does not move.

⚠ **THE EXISTING TEST COULD NOT SEE IT, AND SAID SO IN ITS OWN PROSE.** The
fixture authors `lift_at_s: 0.1` and
`a_motion_is_priced_by_how_much_of_the_gap_it_actually_covers` described it as
*"900px/s for a 0.5s move: about 450px of travel"* — the naive product, read
back as the fixture's description. Reverting the repair leaves that test GREEN.
The new arm is a FLIP rather than a threshold, because the tent
`1 - |travelled/gap - 1|` is wide enough that both distances clear
`MOTION_WORTH_PRESSING` at most gaps: `a_burst_covers_ground_only_after_its_impulse_fires`
probes at 360px and at 450px and asks which one PEAKS. Two poisons, two
different arms — counting the windup fails the flip (0.75 against 1) and
counting ONLY the windup fails the peak (0.25).

⛔⛤ **AND THE GRID SWEEP CANNOT WITNESS IT: 7195% → 7195%, EVERY ROW
BIT-IDENTICAL.** That is a finding about the instrument. The one shipped move
on this road is the medic's, and her row is `1/17/34` — she throws
`medic_tourniquet` 144 times and nothing else, so her lift is never pressed in
a flat-stage mirror and the price never reaches a decision. The band where the
decision actually changes is computable and is stated instead of measured: a
motion is offered while `travelled/gap` sits in `[0.5, 1.5]`, so the old 435px
offered the lift from 290px to 870px and the true 326px offers it from 217px to
652px. A 700px gap was 0.62 (pressed) and is 0.47 (not). Nothing on the grid
stands 700px from a medic.

⛔⬤ **THE NAMED SLICE, AND EVERY PARAGRAPH ABOVE IS EVIDENCE FOR IT: THE BRAIN
IS RECONSTRUCTING COMBAT SEMANTICS FROM WHICHEVER PIECES HAPPEN TO LIVE IN
`MoveSpec`.** Grabs, windboxes, bolts, bombs, ranged triggers, summons and
recovery routes each arrived as a separate patch onto `MoveFrameData`, and the
ranged half was the loudest: `polygon_projectile_charge_shot` was ADMITTED by a
`hazard_reach` of `RANGED_ACTION_REACH = 1000` — a constant wider than any
stage this game ships — and then scored with `coverage: none`, `reach_fit: 0`,
`damage: 0`, `launch: 0`, because its real speed, flight, damage and launch
live in the BODY's `RangedActionSpec` and not in the move at all. Raised by
review 2026-09-20: *"continuing to add exceptions for ranged actions, bombs,
summons, bolts, etc. will create a second approximate combat model."*

✅ **INCREMENT ONE LANDED 2026-09-20: THE HAZARD IS A VALUE, AND THE
PLACEHOLDER IS A REQUEST.** `hazard_reach: f32` is gone;
`MoveFrameData::hazard` is an `Option<MoveHazard>` with two variants —
`Spawned { reach, speed }` for a hazard the catalog can measure whole, and
`OwnersRangedAction` for the one shape it cannot. Three things fall out of the
shape rather than out of a new special case:

* **The speed stops being thrown away.** The old fold computed `speed ×
  lifetime` and kept only the product, so a consumer could ask when a bolt is
  THROWN and never when it ARRIVES. Admission now leads to the arrival:
  `thrown_at + min(gap, reach) / speed`, one fixed-point pass, exact for a
  standing opponent and erring toward under-leading a retreating one — the
  direction that refuses a shot rather than throwing one that cannot land.
  Held by `a_travelling_hazard_is_aimed_where_the_foe_will_be_when_it_arrives`,
  with a stationary-hazard control and a stationary-foe control, because
  *"refused"* has two innocent explanations.
* **The placeholder became a question with an owner.** `OwnersRangedAction` is
  a REQUEST, and `attack_kit_of` — the same layer that already joins a grab to
  its capture params — answers it from the body's `ActionSet`, in the
  runtime's own precedence (what the move EQUIPS, then the body's standing
  kit). Projectile Polygon's cannon resolves to **1306px** (540px/s × 2.4s +
  the shot's own body) against the 1000px constant, and a body carrying no
  weapon leaves the request STANDING rather than having a reach of zero
  invented for it.
* **A key the catalog has not been taught answers `None`**, which the type now
  says out loud where a `0.0` had to be explained.

⛔⛤ **AND THE JOIN GOT THE BOOMERANG WRONG BY A FACTOR OF TWO ON THE FIRST
PASS**, caught by reading the flight's own doc after the sweep. A boomerang is
a constant deceleration to a stop, so `speed × boomerang_return_s` is twice how
far it gets: the displacement is `v0·t − v0·t²/2·out_s`, which at the
turnaround is `v0 · out_s / 2`. The ponytail's 430px/s over 0.34s reaches
**73px** plus the shot's 10px body, not 156. ⇒ The same shape as the whole
slice one layer down — a TIME and a SPEED beside each other are not a distance
unless the motion is uniform — and the hazard's published `speed` is now the
AVERAGE over the outbound leg, so `reach / speed` is the time it actually
takes.

⭐⭐ **JUDGED ON THE GAME: 7059% → 7197% (+2.0%), FOUR OF TWENTY-ONE ROWS
MOVED, AND THE TWO THAT ROSE ARE THE TWO THE PREVIOUS SWEEP LEFT AS OPEN
QUESTIONS.**

| fighter | before | after | moves | distinct | most thrown |
|---|---|---|---|---|---|
| `director` | 34% / 97% | **132% / 129%** | 62 → 129 | 11 → 17 | `director_train_of_thought` ×39 → ×67 |
| `officer` | 78% / 97% | **116% / 127%** | 61 → 96 | 17 → 15 | `officer_the_draw` ×20 → ×30 |
| `npc_pirate_admiral` | 239% / 201% | 245% / 176% | 97 → 92 | 14 → 11 | `run_out_the_guns` ×53 → ×48 |
| `projectile_polygon` | 184% / 229% | 144% / 228% | 121 → 123 | 17 → 17 | `polygon_ponytail_boomerang` ×25 → `polygon_projectile_charge_shot` ×35 |
| the other 17 | — | — | — | — | bit-identical |

⭐ **THE DIRECTOR'S ROW WAS THIS FILE'S OWN OPEN QUESTION AND IT IS ANSWERED.**
The previous sweep recorded him at 34%/97% and said a split like that inside a
MIRROR is the two copies narrowing onto one move; the same paragraph named the
cause — *"a bolt is not a threat where it is thrown … the truthful number is
`throw + gap / hazard speed`, and the hazard's SPEED is exactly what
`MoveFrameData` does not carry."* With the speed carried he fights on 17
distinct moves instead of 11 and throws the bolt 67 times instead of 39.

⚠ **AND THE POLYGON'S ROW FELL, WITH A CAUSE THAT NAMES INCREMENT TWO.** Her
side-B stopped being a stage-crosser: the ponytail's real outbound reach is
83px, not the 1000px placeholder, so the brain correctly stopped throwing it at
range — and fell through to the charge shot, which deals `4` where the
boomerang deals `7`. The option layer cannot see either number: `max_damage`
folds Active volumes and a shot is not one. ⇒ A fighter choosing between two
projectiles on reach and frame advantage alone picks the weaker one, which is
exactly the hole increment two fills, and the sweep predicts its direction.

✅ **INCREMENT TWO LANDED 2026-09-21: THE DAMAGE, AND THE GATE THAT WAS
SWALLOWING IT.** Two halves, and the second exists because the grid measured
the first as inert.

* **The hazard carries what it deals.** `MoveHazard::Spawned` is
  `{ travel, damage }`; `hazard_of` reads `SteeredBoltParams::damage` and
  `DropBombParams::damage`, and `resolve_owners_ranged_action` answers the
  ranged request with the weapon's — the same join one field further, in the
  layer that already holds both halves. `MoveFrameData::max_damage` stays
  VOLUMES ONLY, because the fighter rollout applies it when the foe is inside
  `frames.reach`, which is where a volume lands and not where a shot arrives;
  the scorer asks `MoveFrameData::strongest_hit`, which is the `max` of the
  two and the one place they meet.
* **And `expected_payoff`'s gate asked `startup_s`.** That field falls back to
  the move's whole DURATION when there is no Active window — the shape of
  every launcher — so `their_commitment > startup_s` was asking whether the
  opponent is committed for longer than the attacker's entire animation. The
  gate was structurally shut on every ranged move in the game, and a damage
  that reaches the kit but cannot reach a decision is a mechanism wired to
  nothing. ⇒ It asks when the move CONNECTS: `arrival_of` for a hazard (the
  same answer admission already computes one block down), `threat_live_at_s`
  for everything else, which is `startup_s` for an ordinary strike by
  construction. ⭐ **THE SAME DEFECT AS THE AIM LEAD, ONE FIELD OVER AND ONE
  DAY LATER** — a field derived with a fallback answers two questions, and
  fixing the first reader does not fix the second.

⭐⭐ **JUDGED ON THE GAME: 7201% → 7151% (−0.7%), ONE OF TWENTY-ONE ROWS
MOVED — AND IT FELL.** Three sweeps at `1732c98ed`, 21 mirror matches of 3618
ticks, `NoWindow`, rung 9, this host. Before / damage only / damage + gate:

| fighter | before | +damage | +gate | starts | distinct | most thrown |
|---|---|---|---|---|---|---|
| `projectile_polygon` | 144% / 228% | **104% / 218%** | 104% / 218% | 70+53 → 68+53 | 15+14 → **14+13** | `charge_shot` ×18 → ×19 |
| the other 20 | — | bit-identical | bit-identical | — | — | — |

⛔⛤ **THE FIRST SWEEP IS WHY THE SECOND HALF EXISTS, AND IT IS THE MORE
USEFUL OF THE TWO.** Damage alone moved ONE row, and it did not move it by
pricing a launcher: the boomerang (7), the cannon (4) and the bomb (12) are
all under her largest volume (16), so `kit_max_damage` is unchanged in a
standing kit — but her AERIAL menu tops out lower, and on those ticks the
bomb becomes the kit's largest and re-prices everything against it. ⚠ **A
DENOMINATOR CANNOT REORDER, and that is what identifies the cause**: `power`
is a ratio, so raising `kit_max_damage` scales every attack's payoff by one
factor and attacks are only ever compared with each other. What moved the
decisions is the NUMERATOR — three moves going from a false `0` to their real
damage — and what displaced was her up smash: seat 1 threw it four times for
31 damage and now throws it once for none.

⛔ **AND THE GATE IS BIT-IDENTICAL IN ALL 21 ROWS, WHICH IS A FACT ABOUT THE
CROSSOVER RATHER THAN ABOUT THE FIX.** `connects_at` for her charge shot is
`0.26 + (gap − 10) / 540`, which equals its `startup_s` of `0.58` at a gap of
**183px** — so the new gate is more generous inside that (where these two
spend 1418 of 3618 ticks) and stricter beyond it, and neither direction
changed a decision. The gate only opens against an opponent committed for
LONGER than the arrival, and on this grid that window never co-occurred with
a launcher being the best thing on the menu. Held by
`a_launchers_payoff_is_gated_on_when_its_shot_arrives_not_on_the_whole_move`,
whose two controls are a slow shot (the opening closes before it lands) and
an ordinary jab (unchanged).

⚠ **THE FALL IS REPORTED RATHER THAN TUNED AWAY, AND HERE IS THE JUDGEMENT.**
A scorer reading `0` for a move that deals `7` is not mis-tuned, it is blind,
and no weight fixes a false zero. What the sweep actually shows is that **the
utility weights were fitted while every launcher's payoff was identically
zero**, so switching the term on for a whole class of move is a re-pricing
those weights have never seen. She still fights — 104% / 218% is far above
the 0.5 gate — and the cost is one row and 0.7% of the pool.

⚠ **AND THE NAMED ASYMMETRY THAT WOULD PRICE IT PROPERLY, so the next
increment does not start by re-deriving it.** A launcher has `coverage: None`,
so `reach_fit` is **zero for it at every range** while its `expected_payoff`
is now paid in full — a swing's worth is discounted by the gap and a shot's is
not. That is the term a projectile's value is actually missing, and it is a
new feature (hazard coverage) rather than another scalar.

⛔⛤ **AND THE LAUNCH HALF IS NOT A MISSING JOIN — IT IS A UNIT — WITH ITS
ARITHMETIC WRITTEN DOWN HERE.** A ranged move is still scored `launch: 0`, and
the reason is not that nobody plumbed it: a projectile hit writes
`HitKnockbackMagnitude::FeelScale(0.85)`
(`projectile/systems.rs`), a DIMENSIONLESS multiple of the victim's feel
tuning, while an authored volume writes `LaunchSpeed { base, growth }` in
px/s, which is what `LaunchEnvelope` is made of. Resolved against the shipped
`enemy_knockback_{x,y}` of `360 / 260` — no live ruleset declares its own —
`0.85` is a launch of about **377px/s, FLAT**: larger than every shipped
smash's base (George Booul's forward smash is 185, the largest pulse on the
roster is `bivalence` at 367) and, because the projectile road applies no
percent term, overtaken by them the moment the opponent is worn. ⇒ So the
brain is not merely missing a number, it is missing a CONVERSION, and
`LaunchConditions` — *"everything about the WORLD and the VICTIM that a launch
depends on"* — is the value that does not carry it. Three things to settle
before this lands, and none of them is a scalar: whether the feel reference
belongs on `LaunchConditions`; whether `0.85` stops being a literal in the
projectile stepper (it is one fact with one owner today and that owner is a
`systems.rs` call site); and what `max_knockback` means for a launcher, since
the fighter rollout spends it as `LaunchSpeed(frames.max_knockback)`.

✅ **AND THE THIRD FACTOR THE HIT RESOLVER SPENDS REACHED THE SCORER —
STALING, 2026-09-21.** `LaunchConditions::growth_scale`'s own doc has said
since the launch law was collapsed that it is *"its
`victim_percent_knockback_scale` folded with THIS MOVE'S STALING INFLUENCE"*.
The brain carried the first factor and not the second, and it carried no
damage staling at all: `apply_hitbox_damage` resolves a landing as
`damage × stale_scale(n)`, so on the smash stage's declared
`0.05 / 0.55 / 0.30` a move landed nine times recently deals **55%** of what
`expected_payoff` priced it at. A specification on the type and one term in
the value.

* `AttackCandidate::wear` is a `MoveWear` — `{ damage, launch_growth }` —
  resolved by `attack_kit_of` from the body's own `BodyStaleMoves` ring and
  the stage's `ResolvedCombatTuning`, in the layer that already joins a grab
  to its capture params and a ranged move to its weapon. ⛔ It is NOT in
  `MoveFrameData`: that is a pure derivation of a `MoveSpec`, and two bodies
  holding one moveset wear their moves differently.
* ⛔ **THE TWO HALVES ARE SEPARATE BECAUSE THE RESOLVER'S OWN SPLIT IS.** The
  damage answer is spent whole; the launch answer is that weakening attenuated
  by the declared influence and applied to the PERCENT TERM only, never to
  `base`. Multiplying the whole launch by the stale factor is what once threw
  away half of everything at high percent and stopped the stock ending.
* ⭐ Both the share AND its scale take the wear, because a kit whose best
  answer is worn out has a genuinely lower ceiling.

⭐⭐ **JUDGED ON THE GAME: 7151% → 7096%, THIRTEEN OF TWENTY-ONE ROWS MOVED,
SIX UP AND SEVEN DOWN.** Same instrument and stamp as the two sweeps above.

| fighter | before | after | starts | distinct | most thrown |
|---|---|---|---|---|---|
| `npc_emmy_noether` | 146% / 146% | **193% / 193%** | 48+48 → 50+50 | 11+11 → 10+10 | `smash_forward` ×14 → ×10 |
| `npc_ninja_shadow_oni_leader` | 179% / 182% | **202% / 206%** | 27+27 → 30+32 | 8+5 → 7+9 | `iaijutsu` ×14 → ×19 |
| `projectile_polygon` | 104% / 218% | **144% / 228%** | 68+53 → 70+53 | 14+13 → 15+14 | `charge_shot` ×19 → ×18 |
| `pugnacious_polygon` | 218% / 198% | 230% / 224% | 58+51 → 47+55 | 17+18 → 18+18 | `uppercut` ×10 → `haymaker` ×12 |
| `goblin` | 144% / 119% | 170% / 123% | 65+62 → 68+67 | 15+12 → 15+11 | `dirt_kick` ×38 → ×45 |
| `director` | 123% / 144% | 128% / 155% | 52+52 → 48+48 | 13+15 → 12+12 | `train_of_thought` ×20 → ×21 |
| `npc_oiler` | 224% / 263% | 229% / 217% | 52+56 → 50+52 | 16+14 → 13+13 | `slick_dash` ×9 → `tilt_forward` ×10 |
| `pointed_polygon` | 199% / 218% | 197% / 220% | 58+53 → 56+51 | 14+12 → 14+13 | `rising_edge` ×15 → `point` ×11 |
| `mary_o_tall` | 206% / 244% | 206% / 240% | 65+71 → 60+65 | 13+13 → **16+13** | `slide` ×17 → ×15 |
| `officer` | 116% / 127% | 116% / 127% | 56+40 → 56+40 | 14+8 → 14+9 | `the_draw` ×17 → ×17 |
| `npc_alice` | 178% / 173% | 170% / 170% | 42+45 → 36+33 | 18+17 → 14+10 | `dash_attack` ×5 → `cipher_sweep` ×6 |
| `player_robot_v3` | 225% / 223% | **163% / 179%** | 61+58 → 51+47 | 17+18 → 14+14 | `rocket_dash` ×15 → ×14 |
| `perfect_cellular_automaton` | 201% / 195% | **125% / 103%** | 53+55 → 30+39 | 16+12 → 11+14 | `glider_launch` ×15 → ×9 |
| the other 8 | — | bit-identical | — | — | — |

⭐⭐ **AND THE POLYGON'S ROW CAME BACK BIT-IDENTICAL TO WHERE IT STARTED.**
Against the sweep taken before ANY of this slice, `projectile_polygon` is
byte-for-byte the pre-change run: the 40 points the damage half cost her are
not approximately recovered, they are exactly undone. Her charge shot LANDS —
88 damage across 19 presses — so it stales, and once it does her launchers
price below her up smash again and the original order is restored. ⇒ **The
two halves are one change**: pricing what a move deals without pricing what
repetition costs is half a model, and the half was what moved her.

⚠ **SEVEN ROWS FELL AND THE TWO HARD ONES ARE NAMED.** `player_robot_v3`
(−106) and `perfect_cellular_automaton` (−168) both press LESS (61+58 → 51+47
and 53+55 → 30+39). The net is −55 of 7151, which is flat, and −105 of 7201
across the whole slice. ⛔ **THE READING IS THE SAME AS THE DAMAGE HALF'S AND
SO IS THE JUDGEMENT.** The runtime has staled every landing since the
mechanic shipped; a scorer spending un-staled numbers was reading a price the
game does not pay, and no weight repairs a false input. What the sweep shows
is that **the utility weights were fitted while `wear` was identically 1.0**
— which is now stated in three places for three different features, and is
the case for the ladder rig rather than for another scalar.

⛔⛔ **AND THE LADDER RIG COULD NOT HAVE DONE THAT REFIT — MEASURED 2026-09-21,
AND THE FLAG HAD BEEN INERT ON THAT ROAD SINCE `--ladder` WAS ADDED.** A refit
has to run on the SHIPPED ladder, and `--weight` wrote the LIVE brain's
`cfg.profile`, which `project_authored_fighter_ladder` rewrites every tick for
any fighter whose profile differs from its authored rung. That filter is gone
deliberately — no tick-based filter composes with the disabling component a
candidate session builds behind — so the projection is a CONTINUOUS authority
and a second writer loses within one tick. ⇒ On `--ladder <shipped>`,
`--weight reach_fit=0` and `--weight reach_fit=999` produced byte-identical
bouts, as did `--apm 1` and `--apm 600`; all four equalled each other, because
the only lasting effect was the `FighterState` rebuild the losing write
provoked. The same flags on the engine FLOOR, where the projection returns
early, moved every number (`--apm 1`: `21% : 9%` → `0% : 0%`).

⇒ Fixed by giving the fact ONE owner: with a ladder installed the override goes
into the ladder ROWS (`FighterBrainLadder::rungs_mut`) before the resource is
inserted, so the projection projects the sweep instead of reverting it. After
the change, on the shipped ladder: `reach_fit=0` → `89% : 74%`, `reach_fit=999`
→ `49% : 54%`, `--apm 1` → `0% : 0%`, `--apm 600` unchanged, and `--no-rollout`
correctly unchanged because the shipped rows already carry rollout `0/0`.

⭐⭐ **THE REFIT IS RUN, AND ITS ANSWER IS *DON'T* — MEASURED 2026-09-21.** Six
arms, `--rungs 1,3,5,6,9 --paired --seeds 15` on the shipped ladder, scored on
the only objective the ladder has: does the higher rung outfight the lower one
at every adjacent pair. `--weight-scale` moves a weight relative to each rung's
AUTHORED value, so the ramp survives and the arms differ in one number.

| arm | 3 v 1 | 5 v 3 | 6 v 5 | 9 v 6 | pairs favouring the higher rung |
| --- | --- | --- | --- | --- | --- |
| **shipped (baseline)** | 15:0 | 12:3 | 11:4 | 15:0 | **53 / 60** |
| `kill_potential` x0.7 | 15:0 | 14:1 | 10:5 | 13:2 | 52 / 60 |
| `expected_payoff` x1.5 | 15:0 | 12:3 | 11:4 | 12:3 | 50 / 60 |
| `expected_payoff` x0.5 | 15:0 | 10:5 | 10:5 | 15:0 | 50 / 60 |
| `kill_potential` x1.3 | 15:0 | 12:3 | **7:8 inverted** | 13:2 | 47 / 60 |
| `frame_advantage` x1.5 | 15:0 | 11:4 | **7:8 inverted** | 10:5 | 43 / 60 |

⇒ **No arm beat the baseline and two inverted a rung pair.** The weights fitted
before the launcher-payoff, staling and hazard-damage repairs still order the
ladder at least as well as any single-weight move tried, so those three repairs
did not invalidate them. ⚠ The gap between 53 and 50 is not itself significant
at 15 seeds; the defensible claims are *nothing improved on the baseline* and
*two settings clearly damaged it*.

⛔ **WHAT THE SWEEP DID FIND IS THAT ONE RUNG IS NOT A STEP.** `6 vs 5` is the
only pair that fails to clear the bar in ALL SIX arms, and the only one that
ever inverts; across all 90 paired seeds it favours the higher rung 56 times,
against `3 v 1`'s 90 of 90. Six different weight settings cannot move it, so it
is not a weights problem.

⚠ **AND THE FIRST VERSION OF THIS PARAGRAPH READ IT AS A FACT ABOUT THE 5/6
PAIR, WHICH THE SWEEP CANNOT SUPPORT.** `--rungs 1,3,5,6,9` makes four adjacent
pairs spanning **2, 2, 1 and 3 rungs** — `6 v 5` is the ONLY one-rung gap in
the design, so "the weakest pair" and "the only pair measured at one rung"
are the same set, and nothing here separates them. ⇒ The claim the data
supports is about the ladder's step SIZE PER RUNG, not about rung 6. Telling
them apart costs one sweep: `--rungs 3,4,6,7,8` measures `4 v 3`, `7 v 6` and
`8 v 7` at one rung each, and if those also sit near 56/90 the pair is
innocent.

⭐⭐ **ONE CAUSE IS ALREADY PINNED IN THE TREE, AND IT SHRINKS EVERY STEP.**
All nine shipped rungs author `rollout_depth: 0, rollout_k: 0`
(`game/ambition_content/assets/data/fighter_brain_ladder.ron`, whose header says
so: *"Rollout fields remain zero until rollout fidelity is good enough"*). Two
consequences:

  * `FighterBrainProfile::for_level` — the ENGINE's ladder, used by any game
    shipping no rows — turns L3 on at `level >= 6`. So the one QUALITATIVE step
    the ladder design has sits exactly at the 5→6 boundary, and the shipped
    ladder does not take it. Every shipped rung is L2.
  * `read_weight` rises 0.0 → 1.0 across the rungs and reads like a main
    difficulty axis, and it is consumed only inside `refine_by_rollout`, behind
    `uses_rollouts()`. It reaches nothing. Pinned by
    `read_weight_changes_nothing_while_the_shipped_rows_disable_the_rollout`,
    which found it the same way this sweep works — an arm that came back
    BYTE-IDENTICAL to its control.

⇒ Rung 5 and rung 6 are authored four numbers apart and only **three** of them
can act: reaction 300→260ms, apm 200→240, noise 0.20→0.16. The fourth, read
0.2→0.3, is inert on every rung of the ladder. That is not the whole answer —
`9 v 6` carries the largest inert `read_weight` delta on the ladder (0.7) and
still went 15:0 — but it is the part that is certain, and it says the ladder is
being measured at less than its authored resolution until rollouts land.

⛔⛤ **AND ONE FIGHTER ON THE SHIPPED GRID CANNOT FIGHT AT ALL — MEASURED
2026-09-21, AND STALING DOES NOT TOUCH IT.** `special_patent_clerk` is the
grid's only duel-gate failure, and his row is the same in every sweep above
because it is the same in every sweep: **80 move starts across TWO distinct
moves, 79 of them `synchronize_clocks`, and eleven damage for the whole
bout.** Both seats identically, hitstun `[12, 12]`, zero knockouts.

⚠ **AND HE IS NOT OUT OF RANGE — HE IS ON TOP OF HIM.** `ticks within 60px:
3573 of 3618`, closest 0px, and `synchronize_clocks` is a real strike (damage
8, a 44 × 12 slice at his feet), so this is not the admission rule refusing a
hopeless swing. 80 starts over 3618 ticks is one press every 45 ticks against
a move that runs 0.65s plus a committed tail: **he is inside this move for the
entire match**, which is why only two of his thirty-three authored moves are
ever started.

⭐⭐ **AND HE IS NOT A BROKEN FIGHTER — HE IS A PERFECT MIRROR, ATTRIBUTED
2026-09-21 BY A CONTROLLED PAIR.** The same command, the same commit, the same
host, differing only in `AMBITION_DUEL_RUNG`:

| | rung 9 | rung 5 |
|---|---|---|
| pool | 11% / 11% | **161% / 181%** |
| starts / distinct | 80 / **2** | 49 / **11** |
| `synchronize_clocks` | ×79, **0 damage** | ×26, **71 damage** |
| knockouts | 0 | 3 |

⇒ His down-special works; it lands 71 damage in the bout where the mirror is
broken. **What rung 9 is, is stated in the harness's own doc**: it is *"the
ONLY rung where `execution_noise * interval()` rounds to zero … so the
per-seat cognition seed is drawn and discarded and a symmetric mirror bout is
deterministically bit-identical."* Two identical brains, in identical
situations, with no noise between them, make identical decisions forever — and
this one kit has a fixed point the other twenty do not.

⚠ **SO THE GATE'S ONE FAILURE IS THE GATE MEASURING A DEGENERATE
CONFIGURATION, AND THE GATE IS STILL WORTH KEEPING.** Twenty of twenty-one
rows pass it, in all three sweeps above. ⛔ It is NOT a licence to lower the
threshold: what a lockstep exposes is a brain with no memory of a move that
keeps producing NOTHING, which is the WHIFF half of the whiff/usage-memory
slice — and that half is untouched. ⛔ **STALING CANNOT BE THE ANSWER AND THAT
IS MEASURED, NOT ASSUMED**: `BodyStaleMoves` records what LANDED, his move
lands nothing, so his ring stays empty — and his row is bit-identical across
the staling sweep, which is that reading taken rather than argued.

⚠ **AND THE TELEPORT'S DESTINATION IS STILL MISSING**, which is the other half
the review named: `TeleportParams` carries `behind_nearest_foe`, `behind_gap`
and an aim, and none of it reaches the brain. See the teleport paragraph
below — it is a fact about the MOVE, so it belongs in the same resolved offer
and not in a third price.

⛔⛤ **AND A FOURTH: A MOVE THAT ONLY CARRIES THE BODY IS NOT AN ATTACK, THOUGH
IT IS VERY TEMPTING TO PUT IT ON THE ONE LIST THAT EXISTS.** A teleport crosses
210px and the admiral's shark is a ridable summon with 650px of authority;
`motion_of` reads only the `lift_*` burst and they author none, so they are on
NO list. Admitting them as ATTACKS cost `player_robot_v3` the whole match:
`phase_shift×157` — one teleport every 23 ticks, which is its whole duration —
at a mean gap of **223px** for **0% damage**. A pressed move owns the body
through its recovery and **a body in a move does not walk**, so putting a travel
move on the attack list does not give a fighter a way to close; it removes the
one it had. `pointed_polygon` and `medic` failed identically.

⛔⛤ **AND THE FIRST REPAIR PUT THE SUMMON BACK ON THE ATTACK LIST BY ANOTHER
DOOR, WHICH A REVIEW CAUGHT THE NEXT DAY.** `frame_data` folded a
`SustainedAuthority`'s reach into `hazard_reach`, arguing that a summon holding
ground for `seconds` makes the opponent the same offer a bolt does — and a test
was written certifying it. `call_the_shark`'s own authoring refutes it in as
many words: *"There is no hurtbox on this up-b, it's purely a mobility
special"*; it is a `hitless_special` rather than a strike with an empty volume
list; the summoned shark is `Neutral` and deals no contact damage; and its
`reach` is authored as **half the ride's straight-line distance**, which is a
statement about where the ADMIRAL can go. ⇒ A number describing how far I can
GO is not a number describing how far I can HURT, and a test asserting the
wrong model is worse than no test. Backed out; `pirate_admiral_moveset` now
asserts `hazard_reach == 0.0` and `threat_live_at_s == None` beside the route.

⭐⭐ **AND TAKING IT OFF THE NEUTRAL MENU REDDENED THREE `smash_ride` FIXTURES,
ALL THREE BECAUSE THE FIGHT GOT BETTER — which is this file's own recorded
failure mode, printed in its own words one assertion above two of them:** *"a
perfectly correct rival [looked] like a broken assertion."* Each was measured
before it was touched, and each repair is about what the test CLAIMS:

| fixture | what it measured | what it now measures |
|---|---|---|
| `the_admirals_up_b_summons_a_shark_he_rides_until_he_jumps_off` | net rightward displacement over 60 held frames, in an app with a live CPU rival: `30.8, 61.7, 33.6, −10.3, 1.9, 3.9` — he steers out and is pushed back, aboard the whole way | the rival stands down and the DIFFERENTIAL is asserted — right `+67.5px`, left `−48.3px`. A rival cannot manufacture that and a mount ignoring the stick cannot produce it |
| `two_admirals_ride_their_own_sharks_at_the_same_time` | that a CPU standing on the stage summons a recovery on its own — true only while the move was on the neutral ATTACK menu. **No second rider in 3600 ticks**, against 600 before | the CPU is put where its up-B is FOR: off the platform and below the ledge, so `classify` returns `Recovery`. ⚠ At the ledge's own height it simply double-jumped home and never needed the shark — a correct fighter defeating the premise |
| `the_ride_ends_when_its_lease_runs_out_and_the_shark_leaves` | held NEUTRAL through the lease, so the pair settled onto the platform and the `!grounded` PREMISE of its recovery-charge arm failed | holds UP, which is what a rider waiting out a lease does and what *"fly around using the control stick"* means |

⇒ **A fixture that shares an app with a live CPU is measuring the CPU too.**
All three read as mount defects and none of them was one.

✅ **AND HALF THE SLICE THAT WAS OWED LANDED WITH IT — 2026-09-20.** The block
was stated as *"its score normalises by SPEED against the kit's fastest and a
`Teleport` authors a DISTANCE, so the two cannot go in one `max` until somebody
decides what that ratio means"*. The ratio was the wrong question: the motion
score had no LENGTH in it at all, so the gap magnitude cancelled and a
full-strength recovery was worth the same at 5px as at 280px as at 900px —
`medic_rescue_lift` applies about `(34, -905)` and was priced identically
everywhere. ⇒ `MotionOption` is priced in pixels against the gap. `travel_of`
reads a thrown velocity over the move's own duration, or a summoned RIDE's own
reach for a route that carries without commanding one, and the score is a
symmetric tent over `travelled / gap`: 1 where the motion arrives, 0 at nothing
and at twice the gap alike. Held by
`a_motion_is_priced_by_how_much_of_the_gap_it_actually_covers` — 5px / 450px /
1400px on one kit, with the middle one asserted to be the PEAK rather than
merely above the press threshold.

⛔⛤ **AND THE TELEPORT WAS PUT ON THAT LIST AND MEASURED BACK OFF IT THE SAME
DAY, WHICH IS THE THIRD TIME THIS MOVE HAS TAUGHT THE SAME LESSON.** Pricing
`RecoveryRoute::Teleport { distance }` as travel toward the opponent cost
`player_robot_v3` his match at BOTH prices tried: **27%/22% on 9 distinct with
`phase_shift×186`** under a forgiving overshoot rule, and **32%/39% on 11 with
×54** under the symmetric one, against **225%/223% on 19** with the move
offered nowhere. Two prices, one outcome ⇒ the defect is not the price. The
authoring says what it is: *"Aimed, like every recovery: the stick, then
straight up"* — a teleport goes where the MOVE aims, so pressing it as an
approach moves the robot 210px upward and the gap it was pressed to close is
still there. `carry()` answers the RECOVERY planner's question and a ride's
`reach` happens to answer the approach question too, because a rider steers;
a teleport's does not. ⚠ **WHAT IS OWED:** the destination is a fact about the
move — `TeleportParams` carries `behind_nearest_foe`, `behind_gap` and an aim
— and none of it reaches `MoveFrameData`. That is the resolved-action-offer
slice below, not another price.

⚠ **A FIFTH WAS CHECKED AND IS A NO-OP, WHICH IS ALSO A RESULT.** Admission
asked only the FAR side of a move's region, and an authored strike is a box
hung OUT from the body — `pointed_polygon`'s thrust spans x ∈ [20, 76] — so
`gap <= far` admitted it against somebody standing in the hole in the middle of
it. `MoveCoverage::span_toward` now returns both sides and admission uses both.
⛔ It refuses nothing today: `probe_how_far_out_an_authored_region_begins` says
118 of 340 authored regions begin away from the body and **the deepest begins at
24.0px** — `ADMISSION_SLACK_PX` exactly, before the foe's half-extent is added.
All 21 rows came back bit-identical. It is here so the code and its own
specification agree; it did not fix the fighter that prompted it.

⭐ **THE TWO HALVES OF THAT TABLE ARE ONE MECHANISM READ FROM BOTH ENDS.** Every
row that improved had been LOCKED ON ONE MOVE it could not land and its mean gap
FELL — `synchronize_clocks×160`, `dirt_kick×204`, `rivet_smash×77`, `slide×60`
all fall to a third of their starts while the distinct count rises. Every row
that fell had its most-thrown move change from a poke to something that is not
one, because **refusing the unreachable swing is right and what the fighter
falls through to is what costs it.** Each of those fall-throughs turned out to
be a separate missing datum, and the four paragraphs below are them.

⭐ **MEASURED WITH BOTH HALVES IN PLACE, and this is what (b) is worth.** Mirror
bouts, 3618 ticks, same `AMBITION_DUEL_RUNG` harness:

| pair | before | with (a)+(b) |
|---|---|---|
| `goblin` @5 | 1 distinct, 0%/0%, 0 hitstun, 0 KO | **15 / 15** distinct, 119%/171%, 518/756 hitstun, 2 KO |
| `special_patent_clerk` @9 | 1 distinct, 0%/0%, 0 hitstun | 185%/234%, 316/468 hitstun, 4 KO |
| `npc_pirate_admiral` @5 | 16% total | 60%/103% |
| `npc_emmy_noether` @5 | 36% total | 41%/60% |
| `npc_carl_stargan` @5 | 28% total | 78%/43% |

The ladder still points the right way: `goblin` @9 reads 142%/188% with 3 KO
against @5's 119%/171% with 2 KO, and move STARTS fall from ~200 to ~60 a bout —
the presses that disappear are the ones that could never land.

⛔⬤ **AND (b) IS HELD BACK BY A FIXTURE, NOT BY DOUBT ABOUT THE CHANGE.** With
(b) in, `smash_in_the_host::launched::an_up_tilt_*` both go red, and the reason
is NOT the launch formula. Localised 2026-09-19 by instrumenting every refusal
in `apply_hitbox_damage`: at the moment of the fixture's hand-spawned strike the
victim is inside the volume, present in the victim set, alive, in play, tangible
and not deduped — no guard refuses it — and the meter still reads 0 before and 0
after. ⇒ The loss is DOWNSTREAM of the damage road's guards, and the likeliest
owner is `void_pending_player_hits_at_lifecycle_boundaries`, which clears
pending hits when a fresh attempt begins. The fixture shares its app with two
CPUs that, once they actually fight, can decide the match and start one.

⚠ **THREE FIXTURE REPAIRS WERE TRIED AND ALL THREE ARE REFUTED**, recorded so
the next attempt does not re-walk them:
1. *Stand both seats down with `Brain::stand_still()`* (the idiom the sibling
   `ring_out` fixture in the same file uses) — the victim then never becomes
   `on_ground` at all, settling at (72, 320) with `VICTIM_X` at 520.
2. *Park → update → re-park, to let the published hurtbox catch up* — the
   silhouette was never the problem: it reads exactly the body's AABB, centred
   on the parked position.
3. *Ask the strike's own `HitboxHits` ledger instead of the meter* — this makes
   the "did it land" assertion pass while the launch assertion still fails, so
   the ledger records the overlap EARLIER than the launch is applied. That is a
   weaker guard, not a fix, and it was reverted.

⭐⭐ **CLOSED 2026-09-20, AND REPAIR 1 WAS RIGHT ALL ALONG — IT WAS APPLIED IN
THE WRONG PLACE.** `Brain::stand_still()` on both bodies, inserted in
`two_seated_fighters` after the seats are bound and followed by one
`app.update()`, makes both arms green. Traced by instrumenting every refusal in
the actor damage drain, which printed the whole story in order inside one call
of the settle loop: the attacker took **8** from the victim, the victim took
**14** and then **13** from the attacker, and then **100 from
`LeftTheWorld`** — she had been knocked off the stage. The fixture's own
11-damage strike then arrived at a body that had just lost a stock.

⇒ **PARKING IS NOT STILLNESS, which is the sentence this fixture needed for
three years.** It re-parks the attacker every pass and the victim once, on the
reasoning that two bodies 320px apart cannot reach each other — and a parked CPU
WALKS. The premise held only while the CPUs were harmless; (b) is what stopped
them being harmless. Same class as
`smash_ride::the_admiral_flies_the_shark_around_the_stage_under_his_own_stick`,
whose last assertion is that the mount took ZERO damage *"across a flight in
which nothing struck it"* and which read 9 — exactly one `grapeshot`. Three
fixtures, one premise: **the CPU could not fight.**

⚠ **AND THE ENGINE FACT UNDERNEATH IS WORTH ITS OWN ROW.** The strike published
`LandedBodyHit` and then neither `ResolvedBodyHit` nor `BlockedBodyHit` — a
consumer asking *"did my move connect"* gets a landed hit, an entry in
`HitboxHits` that spends the one-shot slot forever, and no outcome message at
all. Here the cause was benign (the victim was mid-KO), but the SHAPE is the one
both this module and `ring_out` have recorded twice as *"the mechanism is not
established"*, and a landed hit with no outcome is exactly how it looks.

⭐ **THAT POPULATION IS MEASURED AND IT HAS SHRUNK — 164 → 123 OF 470 AUTHORED
MOVES** (`authored_movesets::offer_census`, 2026-09-20). `MoveFrameData::coverage`
was `None` for four unrelated things — a counter, a buff, a launcher whose
damage rides a projectile, and a pure-motion recovery — and the option layer
treated them as one shape. Captures and hazards are out of that bucket now. Of
the 123 left, **all but twelve are taunts, throws and pummels**, which are legal
only while a capture is held and reach this menu never. The twelve are five
`smash_counter::counter_move`, three `smash_vitality` buffs, three that carry
the body, and the Performer's flyline.

⚠ **AND THE CENSUS WAS ITSELF A COPY THAT WENT STALE, WHICH IS THE LESSON.**
Its membership rule was a hand copy of the admission arm, written when that arm
read `lift_speed <= 0.0`; the arm learned `hazard_reach` and the census went on
printing 127 unchanged — a census agreeing with itself rather than with the
engine. The rule is now the arm's rule and the probe's doc says to poison it.

**Next implementation: a brain must be able to DECLINE TO ATTACK, and every
measurement above now points at it.** `wants_attack` takes
`options.attacks.first()` whenever the body is free, so with admission honest
the move that survives at range is whichever one the scorer cannot price — and
whatever survives is thrown until the world changes, which in a mirror it never
does. This is what turns each honest narrowing of the menu into a new lock:
`medic` at 1 distinct move of 17 reachable, 177 presses, zero landed.

⛔ NOT BY A SECOND, SOFTER ADMISSION RULE. A `reach_fit` floor is the same
number the admission constant already owns, and `ADMISSION_SLACK_PX`'s doc says
why those two were one number only because one number was cheaper to write.
What is missing is the brain's memory of its own last move: it cannot perceive
that it just whiffed, so it re-derives the same ranking from the same world.
`FighterState` carries `habits` (a model of the OPPONENT) and `last_foe`, and
nothing at all about what this body just did.

⛔⛤ **AND THE OBVIOUS SHAPE OF THAT REPAIR WAS BUILT, MEASURED AND BACKED OUT
2026-09-20 — RECORDED SO THE NEXT ATTEMPT STARTS PAST IT.** The build was:
`BodyCombat::strikes_connected`, a monotone tally incremented on the existing <!-- cite-ok: built, measured and backed out 2026-09-20 before it was committed; recorded so the next attempt starts past it -->
false→true `connected_hit` edge in `mark_move_playback_resolved_hits` (which is
where `verdict_belongs_to` has already settled provenance, so no second reader
of the channel has to repeat it); published through `PerceptionBody` into
`SelfView`; remembered by the brain as `last_seen_connects` /
`presses_since_contact`; and after three presses with nothing landed, ONE
decision declines to press so movement can close.

⇒ **The decline was a NO-OP, and the measurement says why in one line: the
presses are 24 ticks apart and a decision is 5.** Press rate is bounded by the
MOVE's duration, not by the decision cadence, so the decision the gate skips is
one the body was never going to press on — it is already inside the move it
threw. Subject and control both pressed exactly 10 times in 240 ticks.

⚠ **THE FACT IS STILL MISSING AND THE WINDOW IS THE OPEN QUESTION.** To change
anything the refusal has to last long enough that the body actually walks, and
the non-arbitrary length is the one the move itself names — *spend as long
going somewhere as you would have spent swinging* (`frames.total_s`), rather
than a tick count chosen to make a fighter behave. That needs one more piece of
brain state and its snapshot projection, which is why it is a slice and not a
follow-up line. ⛔ Do not re-derive the plumbing: the edge, the port and the
projection are all named above and all compiled and passed the four suites; it
is the POLICY that was not earned.

⚠ The two moves left after that are a counter and a buff, and they are the two
the attack scorer genuinely cannot choose between: pricing them needs a
DEFENSIVE feature (*"is the opponent committed to a swing"*), not a wider
admission rule. Keep press generation separate from move utility; do not patch
the evaluator with a fighter-specific exception.

**Still open — and it is NOT this lock.** `sanic` @5 takes 0% and always did.
Control run at the parent commit, same harness: the bout ends at **777** ticks
with 5 knockouts and 1275px of axis drift; with (a)+(b), 637 ticks, 6 knockouts,
943px. Both seats are airborne for all but ~200 ticks and throw `spring_launch`.
⇒ That is a body leaving the stage in the first thirteen seconds, a different
defect from the one-move lock, and the grid below lumped them together because
both read as *"under 40% damage at rung 5"*. ⭐ Corroborated 2026-09-20 from the
other side: seated against `smash_george_booul` she deals 17% on 13 distinct
moves and spends 17% of the bout in `Recovery`, so the brain is choosing — the
stage is losing her.

**Acceptance:** representative CPUs select movement-compatible and
movement-transition attacks from their authored menu across the intended
difficulty ladder, with no regression to the press/move identity contract.

⚠ **MEASURED 2026-09-16 — THE LADDER HALF OF THAT ACCEPTANCE FAILS AT RUNG 5,
AND THE FAILURE IS NOT MONOTONE IN DIFFICULTY.** Sweeping `AMBITION_DUEL_RUNG`
over the five published rungs at HEAD, `npc_pirate_admiral` mirror, counting move
STARTS:

| rung | distinct moves (seat 0 / 1) | share that is `grapeshot` + `call_the_shark` |
|------|------------------------------|-----------------------------------------------|
| 1 | 11 / 12 | 48% / 49% |
| 3 | 13 / 16 | 40% / 40% |
| 5 | 11 / **7** | **70% / 84%** |
| 6 | 14 / 15 | 25% / 31% |
| 9 | 13 / 13 | 35% / 44% |

And it shows up in where the damage goes, splitting each `ResolvedBodyHit` on
whether its victim is a seat: rung 5 lands **16 of 105** damage on an opponent
(15%) against rung 9's 51 of 95 (54%). A rung answering 70–84% of its decisions
with two moves and putting 85% of its damage into summons is this acceptance
failing — it is also why
[DUEL-GUARD-RUNG](#duel-guard-rung--the-cpu-duel-guard-fails-at-rung-5-on-main-today)
sees the shipped duel guard go red there, and that row's investigation is what
produced these numbers.

⛔ **NO MECHANISM IS CLAIMED HERE, and one plausible one was checked and
refuted.** `FighterBrainProfile::for_level` switches rollouts on at `level >= 6`,
which would have explained rungs 6 and 9 being the varied ones — but that
fallback does not apply: `ambition_content` ships an `AuthoredFighterLadder`, and
`assets/data/fighter_brain_ladder.ron` authors `rollout_depth: 0, rollout_k: 0`
at **all nine rungs**. ⇒ Rollouts never run in the shipped game, so difficulty is
entirely `reaction_ms`, `apm_cap`, `execution_noise`, `read_weight` and the
utility weights — every one of which the file's own validation requires to be
MONOTONE in level. A monotone ladder producing a non-monotone outcome with a
large outlier at one rung is the finding, and the F6 menu/utility term is where
it belongs.

⛔⛤ **WIDENED THE SAME DAY, AND THE RUNG FRAMING ABOVE IS THE WRONG ONE.** The
bound this row set — *"a defect at one (character, level) pair is not yet a
defect in the rung"* — was the right caution and it was discharged by running
`every_fighter_on_the_grid_can_fight_its_mirror` at rung 5 and at rung 9: 21
fighters, mirror matches, 3600 ticks each, ~12 minutes a sweep. **It is not a
property of the rung. It is a per-(character, rung) LOCK, and it has a
signature.**

| | starts | distinct used | top move | damage | hitstun | neutral |
|---|---:|---:|---|---:|---:|---:|
| `goblin` @5 | 201 | **1** | `dirt_kick`×201 | 0% / 0% | 0 | **100%** |
| `goblin` @9 | 189 | 17 | `dirt_kick`×79 | 138% / 95% | 930 | 61% |
| `special_patent_clerk` @9 | 160 | **1** | `synchronize_clocks`×160 | 0% / 0% | 0 | **100%** |
| `special_patent_clerk` @5 | 143 | 18 | `synchronize_clocks`×52 | 167% / 200% | 1223 | 48% |

⇒ A locked fighter throws ONE move for the whole bout, never leaves Neutral,
and deals and takes exactly nothing — while still starting ~200 moves, so it is
NOT a seating failure and not an idle body. `special_patent_clerk`'s locked bout
re-starts the same move on a median gap of **4 ticks**.

⇒ **AND IT IS NOT "LOW RUNGS ARE WORSE".** `goblin` locked at 5 and was fine at
9; `special_patent_clerk` locked at 9 and was fine at 5. Across the grid, 5 of 21
fighters fell under 40% total damage at rung 5 and 1 of 21 at rung 9 (`sanic`
0→130%, `goblin` 0→233%, `npc_pirate_admiral` 16→87%, `npc_emmy_noether`
36→142%, `npc_carl_stargan` 28→116%).

⛔⬤ **THE SEED EXPLANATION WAS WRONG, AND IT IS WORTH SAYING WHY IT WAS
PERSUASIVE.** This row read the both-directions pattern as
`fighter_cognition_seed` mixing character with level, *"some streams land in a
basin the decision layer cannot leave"*. It is not the stream. Whether a pair
locks depends on whether the pair's equilibrium gap happens to sit inside the
tolerance band of exactly one move whose commitment window covers the decision
cycle — a geometry coincidence, which is why it looked seed-shaped and appeared
in both directions. ⚠ A mechanism that predicts the observed pattern is not
thereby the mechanism.

⭐⭐ **AND RUNNING IT AGAINST A SCORING CHANGE IS WHAT IT IS FOR — 2026-09-19,
`kill_potential`.** The feature was `foe.damage_frac()` alone, identical for
every candidate, and an attack's score is only ever compared with another
attack's (`options.attacks` is read through `first()` and through a lookup by
move id — never against a threshold, never against a movement score). ⇒ **Two
of the five weights every authored rung tunes could not change a decision**:
`kill_potential` and `stage_risk` are both facts about the opponent and about
me, not about the move. `kill_potential` now shares against the kit's best
percent-scaling launch.

⚠ **THE SWEEP IS DETERMINISTIC, AND THAT WAS MEASURED BEFORE THE DELTAS WERE
READ.** Two runs at the same tree agree on all 21 fighters to the printed digit.
Without that control the `used/seen/kit` column looked like it was moving on its
own and every delta below would have been unreadable.

| | before | after | | | before | after |
|---|---:|---:|---|---|---:|---:|
| `npc_carl_stargan` | 116 | **331** | | `perfect_cellular_automaton` | 391 | 205 |
| `npc_emmy_noether` | 142 | 247 | | `npc_alice` | 290 | 150 |
| `npc_pirate_admiral` | 46 | 106 | | `pointed_polygon` | 288 | 192 |
| `sanic` | 80 | 152 | | `npc_ninja_shadow_oni_leader` | 256 | 196 |
| `npc_bob` | 251 | 329 | | `director` | 278 | 248 |
| `performer` | 333 | 421 | | **`medic`** | **188** | **39** |
| `smash_george_booul` | 467 | 536 | | `goblin` (locked) | 24 | 24 |
| `officer` | 198 | 209 | | `special_patent_clerk` (locked) | 0 | 0 |

**Total 4648 → 4833 (+4%). 12 improved, 3 unchanged, 6 fell.** `npc_carl_stargan`
went from **2 distinct moves to 19** — a near-lock of exactly the shape this row
describes, opened by giving the ranking a term that moves as damage accumulates.
Both hard locks are untouched, which is predicted: a locked fighter deals no
damage, so the percent term never rises and the new term never engages.

⛔ **ONE REGRESSION CROSSED THE LINE AND WAS NOT WAVED THROUGH: `medic`,
188% → 39%, 16 distinct moves → 8** — **and both of the readings this row
first offered were wrong.** They were about `medic_tourniquet` losing its kill
credit. Measured instead of guessed, her RANKING is healthy at every position
sampled: jab at 0%, `smash_down` at 150%, `tourniquet` still her answer at a
90px gap. Nothing was wrong with what she picks.

⭐⛤ **THE CAUSE IS THE ADMISSION RULE'S THIRD ARM, WHICH IS HALF OF `(b)`
BELOW.** *"Touches nothing — a buff, a summon, a pure-motion move"* also
admitted a hitless RECOVERY. `medic_rescue_lift` lands no volume, so nothing it
could miss filters it, and with `reach_fit` and `expected_payoff` both zero it
was priced on `frame_advantage` and `stage_risk` alone. Whenever the gap grew
past the rest of the kit's reach it was what remained — and pressing it throws
her 905 units into the air, which WIDENS the gap, so the next decision finds
the same world one recovery later. She threw it 48 times in 91 starts. The
`kill_potential` change did not create this; it moved her trajectory into it.

⚠ **AND THE FIRST FIX WAS MEASURED AND WITHDRAWN, WHICH IS THE PART WORTH
KEEPING.** Excluding every hitless recovery from the neutral menu fixed the
medic (39 → 262, 8 → 20 moves) and cost `npc_emmy_noether` 180% (247 → 67):
her most-thrown move did not change and her GAP grew from 82 to 123, so the
same move is a TRAP for one fighter and the APPROACH for the other. ⛔ A
second finding came out of the same run: the predicate
`recovery_route.offers_a_way_home()` selected FIVE moves, not the two the
comment claimed — `pirate_admiral/call_the_shark` is a `SustainedAuthority`
summon and two more are `Teleport`s, and a summon is exactly what that arm
exists to admit. The census had keyed on `lift_speed`; the filter had not.

⇒ **LANDED AS A LAST RESORT RATHER THAN AN EXCLUSION.** A hitless
self-launcher leaves the menu whenever the menu has something else on it and
comes back when the alternative is an empty menu. Grid sweep: **exactly 1 of 21
bouts moves** — medic 39 → 262, Emmy bit-identical at 247 — and no fighter is
under 40% but the two hard locks. ⚠ It is HALF a repair and says so at the
code: a move whose only effect is to move the body belongs in the MOVEMENT
list, scored by whether it closes the gap, not in the attack list scored by
frame advantage. That is `(b)`'s work.

⚠ **AND THE INSTRUMENT ALREADY NAMED THEM; NOBODY HAD RUN IT.** The sweep is
`#[ignore]`d as *"a measurement, not a guard"*, and its one assertion
(`silent.len() * 2 < ids.len()`) is deliberately about whether the TABLE is
readable, not about whether fighters fight — so one or two locked fighters print
their zeros and it passes. That is the documented design, not a defect in it.

**Next implementation for this half:** the lock predicate (`used == 1` with
`neutral == 100%` over a 3600-tick bout) is no longer the live question — no
pair meets it. What remains measurable is `sanic` @5, and the guard that would
catch it is about a bout ENDING EARLY, not about move variety. ⛔ NOT by
tightening the sweep's existing assertion, which measures something else on
purpose.

⚠ **BOUNDS THAT REMAIN.** Two rungs of the five, mirror matches only, one bout
per pair — and the pairs cannot be resampled, since the seed is
`(character, level)` with no clock. What is NOT bounded any more is the "one
character" caveat: 21 fighters, both directions of the effect.

### ✅ DONE D-POTATO-ASPECT — finish low-tier sprite aspect/trim policy

**Owner:** [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).

**Current state:** systematic downscale/trim generation defects were repaired.
✅ Ruled 2026-10-01 (Q69, [`maintainer-decisions.md`](maintainer-decisions.md)):
character sprites at `potato` stay at 1/16 linear scale, with no fallback to
`0_25x`. That is today's behaviour, so the row closes on the ruling. Potato is
a deliberately tiny caricature that keeps the gist; a comparison against
`quarter` is not an acceptance criterion.

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
   that turns out to be relying on one, and the census above says none is. Filed
   as **`Q138`** in
   [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).
2. ⚠ A guard cannot DECIDE the property, and that stands: "this arm's assertions
   are unsatisfiable by a frozen world" is not readable from source — six
   mechanisms produced it in the table above and a seventh turned up when the
   three unlisted arms were read (`a_move_keeps_its_occurrence_across_a_rewind`
   compares the rollback reading for EQUALITY against a fixed-tick control whose
   own floor is `reached > 0`).
   ✔ **BUT A GUARD CAN STOP THE CENSUS ROTTING, WHICH IS ITEM 0'S WHOLE
   COMPLAINT, AND ONE IS LANDED:**
   `scripts/a_rollback_arm_must_refuse_a_frozen_world.py`. It routes the decision
   instead of making it — a sync-test arm either reads the health API or arrives
   with a sentence naming what a frozen world breaks in it. ⭐ **THIS ITEM IS THE
   CENSUS'S ONE OWNER.** `awaiting-maintainer-decision.md`'s Q138 kept a second
   copy and the two disagreed by 2026-09-18 — it still said 26/15 while this
   said 29/15/12/2 and the guard said 31/17/12/2. Q138 now carries the
   derivation and points here for the count.
   Re-derived **2026-09-18**: **31 fixtures, 17 reading a health API, 12
   adjudicated, 2 not arms** — re-derive
   with `python3 scripts/a_rollback_arm_must_refuse_a_frozen_world.py`, which
   prints the line. ⚠ It reads `git ls-files`, so a NEW fixture is invisible to
   it until staged; both of 2026-09-17's were caught only because the file was
   added before the sweep was believed.
   ✔ **AND THE VERDICT IS NOW HELD BY SOMETHING THAT RUNS** (2026-09-18): until
   then its `main()` was reached only by hand, because `scripts/tests/` tests
   the PARTS and no lane named the script. It is a `--maintenance` job at 12 s.
   The census that found the day's other two unheld ratchets missed this one by
   using `check_*.py` as its population — a scan root is a citation, the same
   defect this guard's own docstring records about `crates/` and `game/`.
   ⛔ **ONE `NOT_AN_ARM` EXEMPTION WAS WRONG IN ITS REASON AND HALF-WRONG IN ITS
   EFFECT.** `game/ambition_app/examples/hall_bench.rs` was exempted as *"it
   asserts nothing"*, which is false at `hall_bench.rs:55`. The exemption stands
   on a different mechanism (`with_required_start_room` guarantees the asserted
   room at tick 0, so a frozen world satisfies it) — but that also makes the
   bench the one caller in the tree that consumes a frozen world SILENTLY, with
   3,300 steps, no health read and no liveness floor. Not a test, so it reds
   nothing; it is still a benchmark that would report the cost of a world that
   stopped advancing.
   ⛔⛤ **THE POPULATION WAS ALREADY FIVE PAST THE 21 THIS ROW CERTIFIED, AND ONE
   OF THEM WAS INVISIBLE TO THE SWEEP.** Four new arms arrived in `game/`, and the
   26th is `examples/capability_demo/tests/rollback_round_trip.rs` — outside
   `crates/` and `game/`, which is where the census looked. It reads the health API
   and is safe, but *"21 arms, all safe"* was measured over a population that never
   contained it. Same defect this row records one section up about `tools/`: a scan
   root is a citation, and a member outside it reads as absent. The guard
   enumerates `git ls-files`.
   ⇒ Item 1 is still the real fix and still wants a maintainer (`Q138`). This only means
   the twenty-seventh arm's author gets the warning item 0 says they do not.
   ⭐ **AND THE TWENTY-SEVENTH ARRIVED THE NEXT DAY AND THE GUARD CAUGHT IT.**
   `cut_rope_arena`'s rewind arm compares a DELTA between two worlds — the beat
   clock against the tick count — which a frozen rollback world satisfies
   perfectly, because it freezes both sides of the subtraction. The only floor
   was on the fixed-tick CONTROL, which is the half that cannot freeze. The
   repair is a liveness assertion the arm needs for its own sake,
   `rollback_ticks >= FRAMES_OF_WAITING`, not a health call bolted on the end.
   ⇒ Item 0's claim is measured now rather than predicted: the twenty-seventh
   arm WAS exposed, and its author got the warning.

<!-- Stated as a field rather than only in prose: two derivations of "what blocks P0/P1" scanned row prose and missed gates recorded only further down. `scripts/check_blocking_set_names_every_gate.py` reads these lines. -->
**Blocked by:** nothing.

⭐ **RULED 2026-09-19 (Q138):** an invalidated harness
must REFUSE or FAIL rather than silently produce frozen observations. ⇒
`Platformer2dSimHarness::step` consults `rollback_health()` and refuses; the
API contract is set. See [`maintainer-decisions.md`](maintainer-decisions.md).

### ROLLBACK-BAG-DESYNC — `AmbitionGameSave` disagrees with its own rollback replay — ✅ REPAIRED 2026-09-16, acceptance MET; the authority/representation split is DEFERRED and Q129 is open

**P0 — ✅ REPAIRED 2026-09-16. The acceptance bar below is MET, including the
half that is not the repro.**

**CURRENT HEAD.** The three live→save mirrors — `persist_inventory_to_save`,
`persist_occurrence_horizon_to_save`, `persist_minted_item_horizon_to_save` —
register through `app.sim_schedule()` and cross the same rollback boundary as
the state they mirror, so a replay re-derives them. No disk I/O moved: they
derive the save RESOURCE from live simulation state and nothing else, and
autosave and the file write stay outside the simulation.

⭐⭐ **TWO MEASUREMENTS, AND THE SECOND IS THE ONE THE REVIEW ASKED FOR.**
1. The set of hashed entries that disagree with themselves across a resimulation
   went from `["ambition_persistence::save::AmbitionGameSave"]` to **EMPTY**.
2. The save's hashed projection went from **1 distinct census across 236 compared
   frames to 236** — against 238 for its busiest neighbours. ⇒ It is no longer
   PINNED, so the empty set in (1) is a comparison that genuinely could have
   failed rather than one incapable of disagreeing.

⚠ **(2) IS WHY (1) MEANS ANYTHING, and neither arm was deleted.** Both arms'
docs instructed deletion on exactly this outcome. They are INVERTED instead:
`no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves` now asserts the
empty set and floors it on the projection varying, and
`the_saves_hashed_snapshot_tracks_the_frames_it_is_compared_at` guards the
un-pinning with a floor of 50 — an order of magnitude above the pinned regime's
1–2 and far below the measured 236. Deleting them would have retired the only
arms that can notice the mirrors drifting back out of the rewind window.

⚠ **ORDERING THAT USED TO BE FREE IS NOW STATED.** While the mirrors sat in
`Update`, `RunFixedMainLoop` running first was what kept a New Game's reset ahead
of them; both are in the sim schedule now, so `.after(reset_inventory_on_new_game)`
is explicit. And the three keep `.chain()` for a load-bearing reason rather than
a tidy one: all three take `ResMut<AmbitionGameSave>`, and an ambiguous relative
order inside a rewinding schedule is nondeterminism the checksum would report as
a desync.

⚠ One behaviour change, stated: the mirrors read the `SaveRestored` latch from a
schedule that runs BEFORE `Update` in the frame, so on the single frame the latch
flips they mirror one frame later. They are value-compared and idempotent, so
that costs a frame of freshness and nothing else.

<details><summary>The defect as it stood, kept because the eliminations below are
what make the repair legible</summary>

**WAS.** `persist_inventory_to_save` runs in ordinary `Update` — once
per FRAME — and writes the live bag into `AmbitionGameSave`, which is
`rollback_resource_clone_checksum` whose projection serialises the WHOLE save. A
rewind re-simulates the sim schedules and does NOT replay `Update`, so the same
historical frame sees a different save. Changing `OwnedItems` once per tick
desyncs a GGRS sync test within six ticks, by the direct write AND by the
sanctioned `ItemGrantRequested` road; the mismatch repeats at frames `[2, 3, 4]`
and presents as a frozen clock
([ROLLBACK-DEAD-SESSION](#rollback-dead-session--an-invalidated-ggrs-session-stops-the-clock-in-silence)).

**CURRENT INVARIANT — what the tree actually guarantees today.** `OwnedItems` is
`rollback_resource_clone` and `feeds_peer_checksum` is FALSE for that kind, so
the bag is restored and never compared; something hashed derives from it. Asked
of the registry rather than guessed: **1 of 364 probed entries differs, and it is
the save.** ⇒ The trigger is the VALUE MOVING, not the writer — a system taking
the same `ResMut<OwnedItems>` in the same schedule position that grants ZERO
ticks 1:1 and stays `Ok`, and `grant(item, 0)` still derefs mutably so change
detection fires identically in both arms.

⭐ **`Update` IS THE WRITER, MEASURED AT THE REWIND BOUNDARY** — the fact a
two-run diff cannot show. `RollbackRestoreAudit` reads frames 2, 3 and 4 each
diverging with the REPLAY xor CONSTANT at `0xce4e4758…` while the first-pass xor
moves every frame: replay re-runs the sim schedule and not `Update`, so every
replay sees whatever the last frame's `Update` wrote.

</details>

**WHAT REMAINS.** ⛔ **NOT "remove `AmbitionGameSave` from the checksum" — that
remedy is REFUSED and this row used to recommend it.** The census invalidates it:
**19 systems take `ResMut<AmbitionGameSave>` and 18 of them execute INSIDE
rewinding simulation schedules** — quests, flags, switches, encounters, shrines,
cutscenes, boss state, the map's visit stamp, the three save mirrors and the
dialogue visit counter. ⭐ **THE NINETEENTH IS `load_save_at_startup`, IN
`Startup`**, named here because *"18 of 19"* otherwise leaves a reader to wonder
whether the holdout is a host-loop writer that could be moved — it is not; it
runs once before any timeline exists.

⚠ **CONFIRMED 2026-09-18 BY A SECOND METHOD, AND THE SECOND METHOD WAS WRONG
TWICE FIRST**, which is worth recording because both errors are the kind that
look like a page being stale. A direct `ResMut<T>` parameter scan found only
**17**: two of the nineteen are `Option<ResMut<AmbitionGameSave>>`
(`session/durable_horizon.rs:509`, `game/ambition_content/src/bosses/cut_rope/mod.rs:301`)
and the `Option<` wrapper sits between the colon and the `ResMut`. Then, with
those included, classifying by schedule gave **19 of 19** — because `Startup` is
neither a rewinding schedule nor a host loop, and a two-way test calls it
rewinding by default. ⇒ The row's own numbers survived both, so the ratio is
now measured by two independent roads.

⭐⛤ **AND A THIRD METHOD READS 23, WHICH IS ALSO RIGHT — THE DIFFERENCE IS THE
QUESTION, NOT THE TREE.** `check_rollback_mutators_run_in_sim.py`'s
`mutating_systems()` reports 23 systems mutably reaching `AmbitionGameSave`,
because it also follows the FIFTH SPELLING its own docstring describes:
`world.resource_mut::<T>()` inside a `fn(world: &mut World)` body, which no
signature scan can see. The four extra are `adopt_loaded_save`, `end_cutscene`,
`process_new_game_reset_request` and `reset_cut_rope_boss_attempt` — every one
an exclusive-world system, and the third is one of Q136's two real
`NewGameResetRequested` producers.

⇒ **This row's 19 is a SIGNATURE count and stays one**, because the sentence it
supports is about systems that declare the save as a parameter. ⚠ Recorded so
the next reader who runs the broad census does not "correct" 19 to 23: the
larger number would strengthen the same argument, and quoting it here without
its method would make two pages disagree about a fact neither is wrong about. Unhashing would make the repro green by throwing away
comparison coverage for substantial simulation state.
⚠⛤ **RE-DERIVED 2026-09-18, AND IT USED TO READ "13 of the 19".** The ratio moved
because the tree did, not because the method did: the three `persist_*_to_save`
mirrors and `track_room_visits` are in the sim schedule now,
`dispatch_pending_dialog_requests` left the population entirely (it no longer
takes the resource), and `count_the_dialogue_visit_when_a_conversation_opens`
joined it inside the schedule. The ONE remaining outsider is
`load_save_at_startup`, in `Startup` — before any timeline exists.
⇒ Method, so the next reader can redo it: `ResMut<'?, AmbitionGameSave>` parameter
occurrences over `multi_writer_resource_census.production_files()` with comments
and test modules stripped (19 occurrences, 17 files, 19 distinct enclosing `fn`s),
each name then resolved to the `add_systems` call that registers it and that
call's first argument read. ⚠ The one-caller indirection matters: `track_room_visits`
is registered with a schedule PARAMETER, and the single production caller
(`progression_schedule.rs:99`) passes `sim`. The full table is in
[Q129](awaiting-maintainer-decision.md#q129--must-the-save-file-be-part-of-what-two-peers-agree-on),
which owns it.
✅ The opposite direction is what LANDED, and it is described under CURRENT HEAD.

⛔ **STILL NOT ATTEMPTED AND STILL DEFERRED ON PURPOSE:** the larger *"is
`AmbitionGameSave` both simulation authority and disk representation"* split. The
review's instruction was smallest-correct-phase-boundary-repair first; that is
done, and the split is a separate piece of work that now has a working boundary
to reason from.

✅⛤ **AND THE OTHER `Update` WRITER IS CLOSED TOO — THIS ROW USED TO SAY IT WAS
"UNTOUCHED" AND "the ONLY hashed-save writer left outside the rewind window".**
`dispatch_pending_dialog_requests` called `increment_dialog_visit` from `Update`,
and an increment has neither property that let the three mirrors move — replay
would double-count it and a rollback loses it. ⇒ It got the different answer it
needed: the count moved into the sim schedule as
`count_the_dialogue_visit_when_a_conversation_opens`, keyed on
`ActiveConversation`'s opening tick, and the restore is what makes it idempotent.
Recorded in
[DURABLE-HORIZON-CHECKSUM](#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update).
⚠ Re-measured 2026-09-18, two ways: `increment_dialog_visit` has exactly ONE
production call site and it is that system, and
`resources_crossing_the_rewind_boundary.py` reports `AmbitionGameSave` **DOES NOT
CROSS** the rewind boundary. ⇒ The count of hashed-save writers outside the window
is **zero**, not one.

**ACCEPTANCE — MET, including the half that is not the repro.** ⛔ *"The startup
repro now passes"* was explicitly NOT sufficient: acceptance also required
showing that a representative IN-SIMULATION save mutation is genuinely being
COMPARED across repeated snapshots, **not that the checksum became accidentally
pinned and therefore incapable of disagreeing**. ⇒ Measured both ways and both
are standing arms: the divergence set is empty, AND the projection moved from 1
distinct census to **236** across the compared frames. The second is guarded at a
floor of 50 so it cannot silently return to the pinned regime.

⚠ **ONE THING THE REVIEW ASKED THAT IS STILL UNEXPLAINED**, and it is recorded
rather than closed: why the mismatch manifested primarily in the opening few
ticks. The repair makes it moot in practice — there is nothing left to mismatch —
but "the first three ticks are special" was never explained, and that is a
property of the rollback window rather than of the save. If it matters elsewhere
it will be found again.

**BLOCKER.** None.
[Q129](awaiting-maintainer-decision.md#q129--must-the-save-file-be-part-of-what-two-peers-agree-on)
remains open for the ownership question, which the repair does not answer and
does not need to.

**Eliminations, one line each, each by measurement.**
⛔ NOT a drained-message edge — `capture_owned_items_baseline` has that shape but
its channel IS `clear_message_on_rollback`, and the census agrees
(`OwnedItemsBaseline` is not among the entries that differ).
⛔ NOT the system's presence — the zero-grant control.
⛔ NOT change detection — both arms mark `OwnedItems` changed every tick; only the
one whose VALUE moves desyncs.
⛔ NOT accumulation and NOT a one-update lag — a pure `f(tick)` write desyncs
exactly as an accumulating one does, and tick 4 is clean where the lag shape
predicts a desync. Both were proposed with opposite predictions, which is the
right shape for a pair of candidates.

⛔⛤ **A FOURTH ELIMINATION WAS A FALSE NEGATIVE AND IT WAS MINE**, kept because
it is the row's most reusable lesson. I reported the save mirror eliminated
because `mirrored_items()` filtered for the substring `"HealthCell"` while the
save stores `PersistedItem { id: "healthcell", count: N }` — lowercase, an
authored id rather than the enum's `Debug` — so it counted 0 while the resource
changed the entire time. ⇒ **A projection that reads nothing and a resource that
holds nothing are the same reading.** ⚠ And it had TWO independent causes: the
row COUNT is 7 in both arms at every step while the summed quantity climbs, so
even the right string would have said "not changing". Finding one cause and
stopping would still have been wrong.

**What holds it in place — and ✔ the good failure arrived, so the arm was
INVERTED rather than deleted.** It said no divergence means the repair landed;
the repair landed 2026-09-16 (`f95d49ce6`, the three live→save mirrors into the
sim schedule). `no_hashed_entry_disagrees_with_its_replay_when_the_bag_moves`
(`which_hashed_entry_moves_when_the_bag_does.rs`) now asserts the diverging set
is EMPTY, still floors the control audit's `resimulations > 0` before reading its
silence, and is paired with
`the_saves_hashed_snapshot_tracks_the_frames_it_is_compared_at` because an empty
set from a PINNED projection is `f = const` and looks exactly like success. A
diverging CONTROL still means the cause is no longer the bag and every
elimination needs redoing. The probes beside it are `#[ignore]`d and print-only,
so they cost the lane nothing.

⚠ **SCOPE.** Measured only for `OwnedItems`; whether other `ResourceClone`
entries behave the same way is unmeasured, and the same probe answers it for any
of them by swapping the system.

Re-run: `cargo test -p ambition_app --test app_it probe_which_hashed_entries -- --include-ignored --nocapture`.

### DURABLE-HORIZON-CHECKSUM — the save mirrors write hashed state from `Update`

**Owner:** `ambition_platformer2d_actor_monolith/src/session/durable_horizon.rs`.

**Current state:** ✅ **BOTH SAVE-SIDE HALVES ARE REPAIRED.** The three
`persist_*_to_save` mirrors AND the dialogue visit counter register through
`app.sim_schedule()`, so a replay re-derives them: the divergence set is empty
and the save's hashed projection tracks (1 → 236 distinct censuses across the
compared frames). ⇒ The repaired mirror half was a per-FRAME write into a
per-TICK checksum, 1 of 364 probed entries differing and it being
`AmbitionGameSave`; measurement, eliminations and reproduction are in
[ROLLBACK-BAG-DESYNC](#rollback-bag-desync--ambitiongamesave-disagrees-with-its-own-rollback-replay---repaired-2026-09-16-acceptance-met-the-authorityrepresentation-split-is-deferred-and-q129-is-open).

✅⛤ **THE LIFECYCLE HALF LANDED TOO, AND THIS ROW WENT ON CALLING IT "STILL
OPEN" UNTIL 2026-09-19.** The restore chain's placement against GGRS start was
[Q135](awaiting-maintainer-decision.md#q135--should-ggrs-start-before-the-durable-restore-has-finished),
and Q135 was **answered and landed 2026-09-16**: `maintain_local_session`
refuses to CREATE a rollback session while `durable_hydration_is_pending(world)`
is true, and the one road that could lower the latch mid-session is gone, held
by a `debug_assert!(restored.0)` in `reset_inventory_on_new_game`. ⇒ The three
`Update` residents below write only while the latch is false, and no timeline
may start in that window. ⚠ The ruling landed on the decision page and nobody
walked its inbound links — the same duplicated-authority drift this row's own
collapse note describes, one document out.

⛔⛤ **AND RE-READING IT AGAINST THE TREE FOUND A REAL HOLE IN THAT ARGUMENT,
FIXED 2026-09-19.** The gate asks for EXACTLY ONE primary body;
`adopt_occurrence_checkpoint_from_save` asked only that the population be
NON-EMPTY. With two primary bodies the gate reports "not pending" and lets the
timeline start, `complete_durable_restore`'s `single()` can never raise the
latch, and the adoption therefore repeats from `Update` on every frame of a live
timeline — writing `AuthoredOccurrences` (checksummed since v195) and both
baselines. ⇒ Its guard now uses `complete_durable_restore`'s own spelling,
`bodies.single().is_err()`, so the gate and all three chain members ask one
question. Held by
`a_population_the_restore_cannot_complete_on_is_written_to_by_nobody`
(`crates/ambition_platformer2d_actor_monolith/src/session/durable_horizon/tests.rs`),
which asserts the gate LETS THE POPULATION THROUGH before asserting nobody
writes — the pair, not the consequence. ⚠ Latent rather than measured: the value
written was constant while the save could not change. What makes it worth the
line is that the invariant holding it harmless was the mirrors' own `!restored.0`
guards, stated nowhere near the write. ⭐ The predicate's own doc had asked for
this — *"keep the two in step"* — and there were three sites, not two.

The ownership
question the save half raised,
[Q129](awaiting-maintainer-decision.md#q129--must-the-save-file-be-part-of-what-two-peers-agree-on),
is still open and no longer blocks anything here. ⚠ Read Q129's pinned-projection
finding before quoting this row's severity: the save's hashed projection took 2
distinct censuses where its busiest neighbours took 238, so the clean runs before
the repair were clean because the comparison was inert, not because the mechanism
was benign.

**What `install_durable_save_horizon` installs — re-derived 2026-09-18** by
reading the three `app.add_systems` calls in its body
(`durable_horizon.rs:349-451`): **nine systems, THREE of them in top-level
`Update`.**

| system | writes | hashed | schedule |
| --- | --- | --- | --- |
| `adopt_occurrence_checkpoint_from_save` | `CustodyBaseline`, `OccurrenceBaseline` | **yes** | `Update` — ✅ pre-timeline |
| `restore_inventory_from_save` | `OwnedItems` + both item baselines | no | `Update` — ✅ pre-timeline |
| `complete_durable_restore` | `SaveRestored` | no | `Update` — ✅ pre-timeline |
| the three `persist_*_to_save` | `AmbitionGameSave` | **yes** | ✅ sim schedule |
| `count_the_dialogue_visit_when_a_conversation_opens` | `AmbitionGameSave` | **yes** | ✅ sim schedule |
| New Game's fresh-run reducers (`start_the_item_domain_fresh`, `adopt_pinned_lifecycle_baselines`) | the baselines, the wallet | yes (baselines) | ✅ the confirmed-frame commit's `CheckpointDomainApply`, since 2026-09-29 |

⭐ **`Update` IS NOT A WAIVER HERE, IT IS A WINDOW.** All three write only
while `SaveRestored` is false, the latch rises once and never falls, and the
Q135 gate refuses to start a timeline while hydration is pending — so these
three run strictly before frame zero of any session. That argument is only as
good as the agreement between the gate's population and theirs, which is why
the 2026-09-19 fix above is part of it rather than a tidy-up.

⚠ **THE HEADER OF THAT TABLE USED TO READ "the five systems this plugin installs
into top-level `Update`", AND IT WAS WRONG IN THREE WAYS AT ONCE** — it counted
the mirrors as `Update` residents after they moved, omitted the visit counter,
and omitted the two reset reducers. ⇒ A count in a table header is a claim about
a population; this one names its method and its reference point so the next
reader can redo it in one command.

⭐ The placement of the three that remain is deliberate and
`runtime/src/durable_save_horizon.rs` says so: *"file/application side effects
themselves are not replayed as simulation ticks."* That argument is sound for the
SIDE EFFECT — writing a file twice is not a desync — and ⛔ silent on the half
that is hashed, which is what Q135 has to settle.

✅⛤ **THE DIALOGUE VISIT IS CLOSED, AND THIS ROW USED TO CALL IT "THE ONLY
HASHED-SAVE WRITER LEFT OUTSIDE THE REWIND WINDOW".** Re-measured 2026-09-18,
two independent ways: `increment_dialog_visit` has exactly ONE production call
site — `durable_horizon.rs:526`, inside
`count_the_dialogue_visit_when_a_conversation_opens`, in the sim schedule — and
`ambition_dialog` no longer names `AmbitionGameSave` anywhere but a comment
recording that it used to. `resources_crossing_the_rewind_boundary.py` agrees
from the other side: `AmbitionGameSave` **DOES NOT CROSS** the rewind boundary.
⇒ The count of hashed-save writers outside the rewind window is **zero, not
one**, and a sentence that says "the only X left" is a hostage the moment X is
repaired.

The findings that closed it are kept, because each is reusable and none is about
the dialogue:

⚠ **THE PREDICTION "NOT FIRST WITH A TEST" WAS WRONG IN A USEFUL WAY.** It said
reaching the increment needs a compiled Yarn project in the harness, because
`dispatch_pending_dialog_requests` early-returns without a `DialogueRunnerEntity`.
True of the DISPATCHER and irrelevant to the question: the question is what the
rewind does to the FIELD, so the arms write `increment_dialog_visit` directly at
each placement and never build a runner. ⇒ **A reachability obstacle in the
writer was read as an obstacle to measuring the write.**

✅ **THE "COUNTED TWICE" BRANCH CANNOT HAPPEN, SETTLED BY READING WHAT DRIVES
IT.** Double counting requires the dialogue START to be replayed through the sim.
Nothing replays it: `DialogState` is a plain `#[derive(Resource)]` with no
registration on any road — re-derived 2026-09-18, no `rollback_resource*`,
`register_rollback*`, `clear_*_on_rollback` or `SessionScopedResources` mention
names it anywhere in the workspace — and the dispatcher consumed the request
with
`state.pending_start.take()` in `Update`. ⇒ On a rewind across the start frame
`AmbitionGameSave` was restored to its pre-increment value, the request that
caused the increment was already gone and did not come back, and nothing re-ran
the dispatcher. **The visit was LOST, full stop** — one outcome, not two.

⛔⛤ **THIS ARGUMENT USED TO OPEN WITH A STRING COUNT THAT WRITING IT DOWN
FALSIFIED, IN BOTH OF ITS TWO OWNERS.** *"`ambition_dialog` contains the string
`rollback` zero times"* stood here and in `Q134`; it contains it twice now, and
both occurrences are inside the comment at
`crates/ambition_dialog/src/bridge.rs:160-167` that records this very finding
and the repair it caused. A crate-wide string count is a fine way to FIND a
road and a poor way to OWN a claim. Corrected in both places 2026-09-18, to the
thing actually checked.

⛔ **AND IT WAS NOT HYPOTHETICAL IN A ROLLBACK SESSION.** `plugins.rs:108`
installs the whole Yarn stack under `#[cfg(feature = "ui")]` and **nothing
else** — it is NOT gated on `simulation_host.is_rollback()`, which is checked
thirteen lines earlier for `AmbitionRollbackPlugin`. The bridge and the rollback
plugin live in the same app in the real game.

⛔ **AND THE FIELD REACHES THE PEER CHECKSUM WITH NO FILTER.**
`AmbitionGameSave::checksum` is `ron::ser::to_string(&self.0)` over the WHOLE
save, so `dialog_visits` is inside the value two peers compare. There is no
projection to narrow and nothing already excludes it.

**The three placements, measured over 200 frames of sync test** in
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`:

| placement of the increment | result |
|---|---|
| `Update` (as it shipped) | the visit is **LOST** (`a_dialogue_visit_counted_from_update_is_taken_back_by_the_rewind`, with a no-rollback control that keeps it 240 frames) |
| sim schedule, 5 known ticks | **exactly 5** (`an_increment_inside_the_tick_is_made_idempotent_by_the_restore`) |
| sim schedule, no rollback session | exactly 5 (the control) |

⇒ **THE RESTORE MAKES AN INCREMENT IDEMPOTENT.** A replayed tick adds to the
snapshot's value, not to what the previous run left, so every replay reaches the
same total. Non-idempotence only bites a write the snapshot cannot reach — which
is where this one was. ⚠ Five ticks rather than one, because one tick reaching 1
is also what "no replay happened" looks like; and the arm asserts
`live_comparisons > 0` so the rewind is a witnessed premise.

⛔ **AND Q134's OPTION 1 IS NOT A REPAIR, WHICH IS A REGISTRATION FACT.**
`install_resource_clone_checksum` installs `rollback_resource_with_clone` and
`checksum_resource` INDEPENDENTLY, so narrowing the checksum changes only what
peers compare and leaves the restore — and the restore is the loss. Measured, not
read: `OwnedItems` is `rollback_resource_clone`, in NO peer checksum, and
`a_bag_changed_from_update_is_silently_taken_back_by_the_rewind` has been green
over its lost `Update` write all along.

✅⛤ **THE REPAIR LANDED THE SAME DAY, WITHOUT THE RULING.**
`count_the_dialogue_visit_when_a_conversation_opens` counts the visit in the sim
schedule, from `ActiveConversation`'s `opened_at == SimTick` — a pure function of
rollback state — chained with the three mirrors for the `ResMut<AmbitionGameSave>`
reason and `.after(interact_ecs_actors_and_switches)`, the system that opens the
conversation. `ambition_dialog::bridge` no longer takes `ResMut<AmbitionGameSave>`
at all, so the presentation dispatcher writes nothing to the save and Yarn stays
presentation-side.

**Acceptance, held by
`a_conversation_opening_counts_exactly_one_visit_across_a_rewound_window`:** two
openings of one node across 240 frames of sync test reach **exactly 2**, with
`live_comparisons > 0` witnessing the rewind. Poisoned both directions —
unregistering the counter gives 0, and relaxing the edge to a level rule
(`opened_at <= now`) gives 6.

⛔⛤ **THE ORDERING IS THE WHOLE REPAIR AND IT COST AN HOUR, SO IT IS WRITTEN
DOWN.** With the opener registered anywhere else in the sim schedule the counter
observes the instance first at `opened_at + 1`, the edge is gone, and the count is
**0** — measured, with the counter running 1186 times, seeing the conversation
live 30 times, and `opened_at == now` never once true. ⇒ A visit counter that fires
on an opening tick is only correct downstream of the opening; the acceptance arm
registers its stand-in in `FeatureInteractionSet::Actuate` because that is where
the real opener sits.

⚠ **AND THE `Update` ARM DOES NOT GO RED, WHICH IS NOT A GAP.**
`a_dialogue_visit_counted_from_update_is_taken_back_by_the_rewind` performs the
increment itself from outside the schedule, so it characterises the SAVE FIELD and
stays green whatever production does. It is the standing witness that this
placement is never available again; the acceptance arm is the one that tracks the
repair.

⚠ **THE LIMIT OF ALL FOUR ARMS.** None drives
`dispatch_pending_dialog_requests` or the real `interact_ecs_actors_and_switches`;
reaching either needs a compiled Yarn project and a body in reach of an NPC. That
the dispatcher is reachable in a rollback session is a source fact
(`plugins.rs:108`, above), and that the counter sits downstream of the real opener
is an ordering constraint the compiler checks, not one these arms exercise.

⇒ What is left of Q134 is a product question (is a visit a fact two peers must
agree on, or per-player progress that should not be in a shared save), and the
defect no longer waits on it.

---

**THE RESTORE CHAIN — the half that was open until Q135 landed.** Kept because
the measurements below are what the ruling was made on; the ruling itself, and
the population hole found while re-reading it, are at the head of this row.

⛔⛤ **THE ONE-SHOT PAIR IS MEASURED AND THE ANSWER IS THE UNFAVOURABLE ONE —
2026-09-16, `probe_when_the_durable_restore_latch_flips_against_ggrs_start` in
`a_bag_changed_mid_window_reaches_the_save.rs`.** Recording, per frame, whether a
session world exists, whether a primary body exists, whether `AmbitionGgrsSession`
is live and whether `SaveRestored` is set:

| | session world | primary body | GGRS live | latch set |
|---|--:|--:|--:|--:|
| first frame true | 1 | 1 | **1 or 2** | **2** |

⇒ **The timeline PRECEDES the write.** A within-frame sampler carrying an explicit
`.after(complete_durable_restore)` edge finds the GGRS session ALREADY LIVE at the
instant the latch has just been set, and **`RollbackFrameCount` reads 1 there** —
timeline frame one, not "before frame zero". The sync-test check distance is four,
so a resimulation reaches back past it, and all three restored resources
(`OccurrenceBaseline`, `CustodyBaseline` and the save itself) are
`rollback_resource_clone_checksum` registrations: a rewind across frame 1 restores
them to their pre-write snapshot and `Update` does not re-run. ⛔ So the
session-scope waivers' *"the write precedes the timeline"* does not merely fail to
transfer — **the opposite is what happens**, and the row's earlier reading ("gated
on the same fact, order stated nowhere") was too generous in one respect and wrong
in another: the two are NOT gated on the same fact. `maintain_local_session`
starts GGRS on `session_world_entity(world).is_some()`; the restore chain waits
for a primary player BODY. The body is the later fact, not the shared one.

⚠ **AND THE GAP IS NOT STABLE AGAINST UNRELATED COMPOSITION CHANGES, which is the
part that makes this a defect rather than a description.** `maintain_local_session`
is in `Update` in `LocalSessionSet::Maintain`, ordered only
`.after(InputSet::Collect)`; the restore chain is in top-level `Update` with NO
edge to it. Adding ONE exclusive system to `Update` — the probe's own sampler —
moved the session start from frame 2 to frame 1 and shortened the boot by a frame:

    without the within-frame sampler   ggrs@2 restored@2, 35 frames, 3/3 runs
    with it                            ggrs@1 restored@2, 34 frames, 6/6 runs

⇒ Each configuration is perfectly repeatable and they disagree, so **the order
these two land in is a property of the whole `Update` set, not of either system.**
The probe perturbs its own subject, and says so in its doc; the exact frame numbers
are not the fact, "nothing orders them" is.

⚠ **AND THE DETECTOR THAT SHOULD SEE THIS IS GREEN FOR A REASON THAT IS NOT
SAFETY — do not read its green as a clean bill.** `OccurrenceBaseline` and
`CustodyBaseline` are value-probed, so
`written_outside_the_rewinding_schedule()` CAN see them, and
`no_registered_type_is_written_outside_the_rewinding_schedule` passes anyway. The
harness boots with no save file, so `adopt_the_ledger` writes the SAME EMPTY VALUE
it found and the comparison is between two identical censuses. ⇒ That is the third
time in one night a control has died of success — the same shape as the repaired
`AmbitionGameSave` positive control and as the presence-probe finding. **The arm
that would demonstrate this needed a SEEDED save — ✅ it is built, and it
desyncs.**

✅⛤ **THE POSITIVE CONTROL EXISTS, and it is the strongest evidence this row
has.** `probe_what_a_mid_session_load_writes_outside_the_rewinding_schedule`
stages a mid-session load at tick 40 on the sync-test harness:

    baseline_rows / custody_rows              1 / 1
    written_outside_the_rewinding_schedule()  ["...continuity::OccurrenceBaseline",
                                               "...custody_horizon::CustodyBaseline"]
    session_health()                          Err("checksum mismatch at frames [38, 39, 40]")

⇒ A REAL DESYNC, at the frames of the load. ⭐ And the attribution is clean: the
staging system writes `AmbitionGameSave` and `SaveRestored` from INSIDE the
rewinding schedule and neither appears in the outside set. What appears is BOTH
baselines, whose only writer here is `adopt_occurrence_checkpoint_from_save`, in
`Update`.

⛔⛤ **AND THE SECOND MEMBER TOOK A SECOND FIXTURE CORRECTION, WHICH IS THE SAME
ERROR ONE LAYER IN.** The first seeded probe passed `Vec::new()` for the custody
half, so that half of `adopt_the_ledger` wrote back what it read and
`CustodyBaseline` stayed out of the set — reported as one defect and one clean.
⇒ **Curing "the harness has no save file" does not cure "the save says nothing
about this field".** The probe now ASSERTS both halves landed, poison-verified to
fail with `(occurrence=1, custody=0)`, so the narrowing cannot recur silently.

⛔ **AND THE FIXTURE SHAPE IS THE PART THAT COST THE HOUR: YOU CANNOT STAGE A
MID-SESSION LOAD FROM OUTSIDE THE TIMELINE.** Writing the save and clearing the
latch between two `step()` calls does nothing at all — measured, the latch never
went false and the save's occurrence count never left zero — because
`AmbitionGameSave` is `rollback_resource_clone_checksum` and `SaveRestored` is
`rollback_resource_clone`, so the next rollback restores both. The staging has to
live in the rewinding schedule, where a resimulation re-applies it. ⇒ That is the
same property the writer under investigation LACKS, which is why the failed
fixture is worth recording beside the working one.

⚠ CONTROL, with its confound stated: the same staging system with the latch left
alone — so the restore chain never fires — reports `Ok(())` and an empty outside
set. Leaving the latch true also lets the in-schedule mirror re-derive the save on
the next tick, so the control differs in two ways rather than one. Enough to
attribute the desync to the chain rather than to a system's presence in the
schedule; not enough to say a save write is harmless on its own.

⛔⛤ **AND OPTION 2 OF THE RULING WAS TRIED, 2026-09-16: IT IS NECESSARY AND NOT
SUFFICIENT.** Registering the three `Update` systems in the sim schedule instead
empties the outside set — `outside_after=[]`, both baselines gone — and leaves
`session_health()` at `Err("checksum mismatch at frames [38, 39, 40]")`. ⇒ **The
placement is not the only cause, and a repair that only moves the systems would
close the detector while the desync survives**, which is the most expensive kind
of green.

✅⛤ **AND THE MEASURED DESYNC IS CLOSED, 2026-09-16, BY A CAUSE THAT WAS NOT A
PLACEMENT: `AuthoredOccurrences` WAS NOT A DERIVED RESOURCE.** The row's earlier
chronology is in git; what it established is that the placement was never the
whole cause.

The cause, found by censusing all 364 probed entries per PASS of each frame — the
arm is `a_mid_session_load_does_not_reach_back_across_the_rewind`,
`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`. When it
was written it read exactly one entry disagreeing between two passes of one frame
— `AmbitionGameSave`, at frames 38 and 39 — and **neither baseline did**, so the
`OccurrenceBaseline` attribution this row first reached by reading declarations
was wrong about the road while right about the resource:

    tick 37   (0,0) (0,0) (0,0) (0,0) (1,0)      (AuthoredOccurrences rows,
    tick 38   (0,0) (0,0) (0,0) (1,1)             save occurrence rows) per pass
    tick 39   (0,0) (0,0) (1,1)                   BEFORE the registration below;
    tick 40   (0,1) (1,1)                         ticks 37-39 now read (0,0) flat

`AuthoredOccurrences` was `declare_rollback_derived_resource` — in no snapshot,
restored by no rewind — justified as *"republished from live state while its room
is loaded"*, and that assertion was false. Two shipped writers make it false,
neither of them a save load:

- `process_new_game_reset_request` calls `forget_everything()` from INSIDE the
  rewinding schedule (`ResetProcessing`), so New Game mutates the ledger
  authoritatively and a rewind across the clear cannot undo it;
- a `Placed` row for a room that is not resident has **no live producer**, so
  there is nothing the republish argument could call on to rebuild it.

⇒ **Measured minimally: ONE `adopt_rows` call inside the schedule desyncs the
sync test at frames 3, 4 and 5 for a write at tick 5** — no save file, no load,
no New Game — and the timeline stalls at `SimTick` 7 where an unseeded run
reaches 31. Held by
`one_write_to_the_occurrence_ledger_does_not_desync_the_sync_test`.

⭐ **AND THE TYPE HAD ASKED FOR THIS ITSELF.**
`AuthoredOccurrences::rewind_argument` said *"if a non-rederived whereabouts
state gains a producer, this ledger must become registered value state with a
value-sensitive probe"* — and `adopt_rows`, 100 lines above it in the same file,
was already that producer. The contract named its own trigger and missed the one
it had.

✅ **THE REPAIR:** `rollback_resource_clone_checksum::<AuthoredOccurrences>`,
projected by `peer_stable_checksum` (a domain-separated fold over `(SimId, whereabouts)`;
the `BTreeMap` order is what makes it deterministic). Entity-free, so a clone
snapshot is the whole story. A declared wire-format change:
`GGRS_ROLLBACK_SCHEMA_VERSION` 194 → 195, baseline rewritten, and it is real
mechanical growth rather than 193 → 194's prose-only bump. Poison-verified —
reverting to `Derived` restores the desync and the tick-7 stall. ⭐ The census
that pointed at it named the ledger at ticks `[37, 38, 39, 40]` beside the save's
`[0, 38, 39]` — **the cause leading the effect by a frame.**

✅ **AND IT CLOSED THE SEEDED-LOAD DESYNC TOO:** after it, **no entry of the 364
disagrees between two passes of one frame** outside world construction. The
seeded-save arm is inverted and kept as
`a_mid_session_load_does_not_reach_back_across_the_rewind`.

⛔⛤ **AND THE REASON NOBODY SAW IT IS A GAP WITH A NAME.** `AuthoredOccurrences`
is probed and is one of the 364 — its probe was PRESENCE-ONLY, and on a RESOURCE
that reports `count: 1, xor: 0` whatever it holds. `declare_rollback_derived_*`
takes a `reason` that is *"an assertion about behaviour"*, and
`every_presence_only_probe_is_named_with_its_reason` deliberately excludes
derived registrations because that reason lives at the registration site. ⇒ **The
promise is checked for EXISTENCE and never for TRUTH**, which is the second time
that has cost this repo a bisection — the registration doc records the first
(`ProjectileOwner`) and predicted this one: *"a derived declaration that lies is
worse than no declaration, because it satisfies the coverage sweep."*

⛔⛤ **AND THE INTERMEDIATE INSTRUMENT IS GONE, DELIBERATELY.** Strengthening the
probe while keeping the type `Derived` was tried first and required a new
registrar variant, `declare_rollback_derived_resource_probed`, across four <!-- cite-ok: deliberately names the API this row records DELETING -->
layers: the obvious route (`..._resource_state`, whose probe is built from
`SnapshotState`) reddened `rollback-wire-format-changes-are-declared` correctly,
because its population is every type that HAS an encoding and a new encoder is a
declared schema change. ⇒ **An encoding is a promise to a peer; a census fold is
not.** But the real answer promoted the resource instead, which left that variant
with zero callers, so it was deleted from all four layers rather than kept as
scaffolding that reads like architecture. ⚠ Verified 2026-09-18: the name has no
hits outside this row. The rule it was built to respect is the durable part and
still holds.

⛔⛤ **WHAT REPLACED THE DIVERGENCE IS A LOST WRITE, AND THAT IS THE HALF STILL
OPEN.** `adopt_occurrence_checkpoint_from_save` runs in top-level `Update`, so now
that the ledger rewinds its write is restored away like any other `Update` write
to rollback state: from tick 40 the save holds the row in every pass (`saved = 1`)
and the ledger holds none (`authored = 0`, in every pass). **A divergence became a
deterministic loss** — two peers agree, and the durable restore does not reach the
ledger under a rollback host. ⚠ No behaviour that worked was taken away: this road
desynced the sync test at the frames of the load, so there was no run in which it
worked, and a fixed-tick host has no restore to lose.

⇒ **THE REMAINING QUESTION IS LIFECYCLE, NOT CHECKSUM:**
[Q135](awaiting-maintainer-decision.md#q135--should-ggrs-start-before-the-durable-restore-has-finished)
— may a synchronised timeline begin before the world it synchronises has finished
loading? Q129 and Q134 ask what belongs in the peer checksum; this asks something
else. The preferred direction is to gate the local GGRS start on durable hydration
rather than to teach the mid-session road to survive a rewind, and the first thing
that owes an answer is whether `adopt_occurrence_checkpoint_from_save` has any
production customer at all. The arm says so at its own definition: when the
`authored = 0` half changes, the lifecycle repair has landed.

⚠ **And the sixth system on that same `.chain()` already carries a partial
waiver saying this.** `restore_inventory_from_save` is waived "FOR THE ACTIVATION
CASE ONLY, AND THE OTHER CASE IS OPEN" because `durable_horizon.rs` supports a
mid-session load — so the mid-session half was known to be open for one member of
the chain and was never asked of the other two.

**Receipts, one line each.**
⭐ This class was PREDICTED in the registration that makes it checkable:
`ambition_persistence/src/rollback_registration.rs` gave `AmbitionGameSave` a real
content projection because *"the ~6 systems that pair a non-rewinding `Local`
edge-detector with these very resources would have failed SILENTLY once rollback
went live."* The instrument existed; what was missing was an arm that makes the
value CHANGE mid-window.
⛔ A claim I made here and withdrew within the hour: that the mirrors write
nothing until a save is RESTORED, so a harness booted without a save file
measures nothing. Wrong — `complete_durable_restore` asks
`ready_body.single().is_err()` and nothing else, so the latch is about a BODY.
I inferred it from the system's name and its `save` field without reading the
guard clause.
⭐ Which is why green sibling arms are not evidence: `rollback_full_reset.rs` and
`rollback_lifecycle_reset.rs` drive these mirrors for 180 and 240 frames, but the
mirror is value-compared, so in a world where the bag never changes it writes once
and returns early forever.
⛔ The premise that caught a bad probe was the SECOND one written: the first asked
`count > 0`, which the STARTER BAG satisfies on its own and would have passed
without the system ever running. Two samples, the later required to exceed the
earlier, is what turns "the value is nonzero" into "my system ran".
⛔ Superseded framings, both measured and both wrong before the third: "it is the
cadence" and "it takes a sustained change, not a single one". It is the START
TICK; the full sweep is in Q129.
⛔⛤ AND THIS ROW ITSELF CARRIED A DUPLICATED AUTHORITY — 2026-09-18, a
97-line block (the one-shot pair through the ruling) appeared VERBATIM TWICE,
`diff`-identical, and the `AuthoredOccurrences` closure was narrated twice with
different emphases. The row was 519 lines and is 412 with nothing dropped.
⇒ A row that grows by appending a correction paragraph eventually re-states its
own middle, and the copy is invisible top-to-bottom because each one reads
correctly in place. ⚠ AND THE SWEEP SAYS IT WAS A ONE-OFF, which is the half that
keeps this from becoming a campaign: hashing every 20-line non-table window of all
129 planning pages finds **zero** duplicated windows after this collapse, and at a
6-line window over all 307 pages under `docs/` exactly two, one a real copy-paste
in `brainstorms/good_llm_ideas.md` (collapsed) and one a code sample that
legitimately repeats a despawn loop in two match arms. ⇒ The instrument is cheap
and the population is clean; re-run it when a row passes ~300 lines.

**Acceptance:** Q129 is answered and the three mirrors follow the ruling; ✅ the
dialog increment has its own answer, which was not the mirrors'; ✅ the one-shot
pair's ordering against GGRS start is characterised rather than assumed; and ✅
Q135 is answered and the restore chain's three `Update` residents have their
road — the session-start gate, plus the 2026-09-19 population fix that makes the
gate's promise hold for every population rather than for the singleton one.
⇒ **WHAT THAT LEAVES IS ONE CLAUSE, AND IT IS ONE THIS ROW SAYS IS NOT A GATE.**
The body states Q129 *"is still open and no longer blocks anything here"*, and
the blocking set's framing agrees, placing it among the sub-road balance calls.
So the row is acceptance-complete but for a ruling that does not block it; ⚠
whether that closes the row is a maintainer call and is deliberately not taken
here. ⚠ The
guard stays banked-but-owed on all of these — see
[ROLLBACK-MUTATOR-POPULATION](#rollback-mutator-population--the-mutator-guard-sees-a-quarter-of-rollback-state)
— which is correct, and is why they were not waived to make a count go down.

### MENU-RESET-MIDSESSION — the menu writes rollback state from `Update` — CLOSED 2026-09-19

**Receipt.** Menu presses reach the simulation through `HostIntentLedger<M>`
(`crates/ambition_platformer2d_actor_monolith/src/session/host_intents.rs`):
the writer stamps the next `SimTick`, and `release_host_intents` writes the
message at the head of that tick in the first pass and in every resimulation
(schema 258 → 259). New Game writes `NewGameRequested`. A Health/Mana Cell
writes `ItemUseRequested`, and `apply_player_heal_requests` runs in
`GameplayEffects` after `apply_item_uses`, so it is after every heal writer.
Ruled (`Q140`): one frame of stale UI is acceptable. ⛔ Do not add duplicate
authoritative inventory state or optimistic reconciliation to hide it.
Witnesses: `a_new_game_asked_for_by_the_host_commits_once_under_a_rewind` and
`a_health_cell_used_from_the_menu_heals_once_and_spends_one_cell`. ⚠ The chest
heal road (`open_ecs_chests`) has no rewind witness yet.

### GUARD-CORPUS / ORPHAN-ARMS — CLOSED 2026-09-16

**Receipt.** `scripts/lib/test_paths.py` is the one answer to "is this Rust
file test-only?". It resolves each file through the `#[cfg(test)]` on its
parent's `mod` line or its own inner `#![cfg(test)]`, not by name, and every
consuming check carries a `POPULATION_FLOOR`. `scripts/tests/test_test_paths.py`
asserts that the orphan list is empty.

### TEST-LANES — keep required test lanes executable

**Owner:** test runner / app integration lane.

⛔⛤ **OPEN, AND IT IS A RED NOBODY OWNED UNTIL NOW: THE COMPILE-COST RATCHET HAS
BEEN FAILING THE FULL GATE AND NO PAGE SAID SO.** Measured 2026-09-18 on a full
`./run_tests.sh` — `12/14 jobs passed in 2212s`, exit 1, and the second failing
job is `compile-cost ratchet (frozen weights, not a stopwatch)`
(`scripts/compile_ratchet.py`). Its baseline records its own provenance as
`b3bd00a4a`, **2026-09-05**
<!-- cite-ok: quoted BECAUSE it resolves nowhere — it is `dev/compile_ratchet_baseline.json`'s own recorded `commit` field, and its unreachability is the finding rather than a fabrication -->
— ⚠ a sha this repository can no longer reach from any ref, because the git
epoch was cut the next day (`b924f419c`, 2026-09-06 03:15). The object survives
in one checkout's object store as a dangling commit and is simply absent in
another, so nothing can recompute what that baseline measured. Five numbers are
outside budget:

```text
largest_unit_lines    ambition_platformer2d_actor_monolith  100,742 -> 115,105  (budget ±2,014)
edit_cost_lines       ambition_platformer2d_actor_monolith  285,213 -> 337,389  (budget +5,704)
edit_cost_seconds     ambition_platformer2d_actor_monolith  1,216.8s -> 1,423.8s
worst_edit_cost_lines ambition_geometry                     540,227 -> 663,508  (budget +10,804)
worst_edit_cost_secs  ambition_geometry                     1,702.5s -> 2,032.5s
```

⛔ **DO NOT RE-FREEZE TO GO GREEN.** The tool offers exactly that — *"if this is
a deliberate landing, say so and re-freeze"* — and banking thirteen days of
growth as the new normal is how a ratchet stops being one. The open question is
which of the +14,363 lines in the monolith's largest unit is a module that
belongs in its own crate; `ambition_geometry` reaching **94.9% of the workspace**
as an edit-cost blast radius is the sharper half.

⚠ **AND THE INSTRUMENT PRINTS THREE OF ITS OWN DEFECTS, which is the reason this
is a row and not a one-line fix.** The baseline disagrees with itself in three
places — `worst_edit_cost` stores 540,227 lines for `ambition_geometry` while its
own `crates` table says 592,091, and the two watched `edit_cost` entries
disagree by 9,114 and 51,385 — so part of every delta above PREDATES the
baseline it is compared against. It also says its un-adopted numbers were
measured at `11ef33c5b5a5` rather than at the frozen commit, so `--diff`'s range
understates where to look. ⇒ Re-freezing would also bank the instrument's
disagreement with itself, permanently.

⛔⛤ **AND THE OTHER RATCHET IN THAT LANE WAS MEASURING NOTHING AT ALL — FOUND,
CAUSED AND FIXED 2026-09-18.** `--maintenance`'s doc-link job scored **0 broken
links for all thirteen tracked crates against a banked baseline of 141**, marked
every row *"⭐ repaired"*, and advised `--update`, which would have written an
empty baseline and retired the ratchet. The cause is the lane itself:
`scripts/run_tests.py:2394` exports `CARGO_TERM_COLOR=always` to every child
job, so rustdoc writes `ESC[1m ESC[33m warning ESC[0m: unresolved link to …`
and the guard's `^warning:` anchor matches nothing. Same crate, same target
directory, one minute apart: `cargo doc -p ambition_characters --no-deps`
printed 21 warnings bare and 21 uncountable ones under the variable.

⛔⛔ **AND THE SAME ANCHOR IS IN THE WORKSPACE WARNING GATE, WHOSE ONLY CALLER IS
THAT RUNNER.** `check_no_warnings.py` matches `^path:line:col: warning: …` on
`--message-format=short`; under colour cargo writes
`src/lib.rs:1:18: ESC[1m ESC[33m warning ESC[0m: unused variable`. Confirmed on
a one-file probe crate: the pattern matches the plain form and not the coloured
one. `scripts/run_tests.py:524` is the only place that invokes it, so this gate
has been reporting clean from inside the lane regardless of the build.
`measure_per_crate_warnings.py` carries the third copy of the anchor.

⇒ **ONE OWNER NOW: `scripts/lib/cargo_output.py`** — `plain_env()` and
`COLOR_NEVER` stop cargo colouring, `strip_ansi()` makes the reading survive a
colour source they do not reach (`RUSTDOCFLAGS=--color=always`, a pty wrapper).
All four consumers read it, and
`scripts/tests/test_cargo_diagnostics_are_read_plain.py` MEASURES the population
— every non-test script that invokes cargo and anchors on `warning`/`error` — so
a new one joins by existing. Poison-verified arms in
`scripts/tests/test_doc_link_ratchet_reporting.py` (19),
`scripts/tests/test_no_warnings.py` (8) and
`scripts/tests/test_a_failed_job_records_what_failed.py` (14), every fixture
COPIED FROM REAL COLOURED OUTPUT rather than composed.

✔ **VERIFIED IN THE LANE, WHICH IS THE ONLY PLACE THE DEFECT EXISTED:
`--maintenance` is `18/18 jobs passed in 460s` with the doc-link job printing
`TOTAL 141`** — the first run in which that job has measured anything inside the
runner. ⚠ A guard going green on a clean tree proves nothing on its own, so the
warning gate was separated from its own repair with a defect it had to see: an
unused import planted in `crates/ambition_geometry/src/lib.rs`, same command and
same variable one minute apart, gave `exit 0 "compiled with no warnings"` before
and `exit 1 "lib.rs:24:5: unused import"` after.

⚠ **THE CLASS ARM IS WHAT SURVIVED NOT KNOWING THE CAUSE.** Two arms written for
mechanisms I could name (cargo exited non-zero; a crate never documented) did not
fire either time — cargo exited 0 and printed its per-crate lines. The arm that
refuses *"every tracked crate at zero against a banked baseline"* caught it
twice. ⛔ And the operational rule I wrote from the first catch — *"do not run
`--maintenance` beside a cargo build"* — was **wrong and has been removed**: both
occurrences did have a concurrent build, but what they shared was `--maintenance`
itself, which is where the variable comes from. The full account is in
[`../recipes/checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md).


**Operational rules, the standing prohibitions and what a green lane does NOT
clear now live in
[`docs/recipes/running-the-heavy-app-it-lane.md`](../recipes/running-the-heavy-app-it-lane.md).**
They left this row on 2026-09-16 because the queue is for open executable work,
not for the rules a closed investigation leaves behind.

**Closed 2026-09-16 — seven arms red, two roads, one symptom. Receipts only.**
Body-relative input under non-down gravity produced zero velocity in five `app_it`
arms and two `--workspace --lib` arms, and this row filed them as ONE cluster
because they shared a symptom. **A symptom is not a population.**

- the five `app_it` arms: fixed at `ce6ddcb25` (the harness stamped
  `control_frame_modes: Default::default()` over the configured mode on every
  scripted step). VERIFIED HERE, not taken on report — `app_it` → 692/0/41,
  274.80 s at `8a15b357b`;
- the two lib arms: fixed at `7ab0fe827` (two fixtures configured the policy on
  the table where nothing reads it). VERIFIED HERE — `--workspace --lib` →
  **7190 passed / 0 failed / 3 ignored** at `8e86a8b8b`;
- ⚠ my red was reported from a tree TWO COMMITS STALE. `git fetch` before
  REPORTING a lane, not only before pushing;
- ⛔⛔ and a RED `cargo test` lane reports a SMALLER POPULATION than a green one —
  49 binaries vs 79 on this box, thirty crates never run. The rule and the
  measurement are in
  [the lane recipe](../recipes/running-the-heavy-app-it-lane.md);
- ⭐ the acceptance criterion that caused both halves is now a standing principle:
  [an acceptance criterion that counts the OLD road's absence](decision-principles.md#an-acceptance-criterion-that-counts-the-old-roads-absence-is-satisfied-by-breaking-the-new-one).
- ⛔⛤ **AND THE FIX FOR THE FIVE MADE AN OPTIONAL RESOURCE REQUIRED, WHICH
  REDDENED A DIFFERENT CRATE'S EXIT GATE FOR A DAY.** `ce6ddcb25` read
  `resource::<UserSettings>()` in `Platformer2dSimHarness::seat_frame_modes`, and
  `crates/ambition_sim_harness/tests/composes_below_the_app.rs` — the Track-4
  gate whose whole subject is that the harness composes with ONLY the reusable
  engine surface — panicked in both arms with *"Requested resource does not
  exist"*. `cargo test --workspace` runs it, so the Rust lane carried it.
  ⇒ Fixed 2026-09-16: the read is optional and falls back to
  `ControlFrameModes::default()`, which is the answer
  `SeatControlFrameModes`'s own doc already gives for an unwritten row; setting a
  preference still requires the resource, and now says so. Poison-verified BOTH
  ways — forcing the fallback reproduces `ce6ddcb25`'s exact 1-passed/5-failed
  signature, and restoring it gives 6/6 plus 41/0 across the harness crate.
  ⭐ **The method lesson is the one this row already carries in another costume:
  a fix verified by the arms that MEASURED the defect is verified against the
  wrong population.** Those five arms went green and the change's reverse
  dependency closure was never run.
- ⛔⛤ **AND A SECOND OF EXACTLY THAT SHAPE, THREE DAYS OLD, FOUND IN THE SAME
  `cargo test --workspace`.** `apply_feature_hit_events` took
  `Res<PlayerDamagePolicy>` — required since `ef8ab19ff` (2026-09-13) — while its
  own comment beside it read *"`Option` so minimal headless test worlds that
  never stand up settings still run at the neutral 1.0"*, and the type's
  `Default` doc promised the same. `ambition_demo_mary_o`'s
  `her_spark_damages_a_snake_through_the_shared_hit_pipeline` builds its App by
  hand, adds that system directly, and panicked *"Resource does not exist"* on
  every run for three days. ⇒ The fixture could NOT be repaired instead: that
  demo's manifest states the E9 oracle outright — *"a downstream game names
  `ambition_platformer2d` + `bevy`, and NOTHING ELSE"* — and the facade does not
  export the policy, so a composition that cannot NAME it has to run without it.
  Fixed at the parameter (`e9f0346d8`), poison-verified both ways.
  ⭐ **TWO DETERMINISTIC REDS IN ONE WORKSPACE RUN, NEITHER INTERMITTENT, NEITHER
  NOTICED** — because `cargo test --workspace` is the only lane that runs either
  arm and nobody had run it to completion. *"Fails in company"* and *"fails
  wherever nobody looks"* are different problems; the triage page for the first
  now says so.

**Current state:** the lane RUNS. `cargo test -p ambition_app --test app_it` →
**713 passed / 0 failed / 45 ignored**, 420.09 s at `e18abd272`, 2026-09-17, on
the CalculexAmbition box with the tree settled. ⚠ **THE POPULATION GREW BY 22 AND
THE IGNORED SET BY 4 SINCE THE READING THIS LINE USED TO CARRY** (691/0/41 at
`041b07158`, 250.90 s, ToothbrushAmbition box) — two boxes, two campaigns'
worth of new arms, and the wall-clock is not comparable between them. Earlier
that night, same box: 690/1/41 (see the open item below), 690/0/41, 688/0/35,
683/0/31, 677/0/25. Missing prerequisites are reported as incomplete rather than
pass.

**Closed 2026-09-16, receipts only — the stories are in git:**
- the long-running `app_it` runaway was a sim-schedule CYCLE, not a flake:
  fixed `23f786757`, capability-guarded `582186bff`, two-sided regression test
  `2e88670d1`;
- the composition probes really STEP (`582186bff`): `step_the_fixed_schedule`
  pins `ManualDuration(1/60)` AND asserts a `FixedUpdate` counter is non-zero,
  because the pin alone returns the arms to certifying a build and that failure
  is SILENCE. Before it: 13 MB, 0.47 s, ZERO fixed steps;
- the `BodyWallet` red (`4ccfef59c`) was a CROSSING, not a schedule choice:
  `NewGameResetCommitted` is produced in the sim schedule and is
  `clear_message_on_rollback`, so no waiver existed.

✔ **TWO SETTLED-TREE `cargo test --workspace --no-fail-fast` RUNS — 2026-09-17,
`a0ac8c7a7`: 8,216 passed, 1 failed; and `f25e21458` after the carrier-order
rebase: 8,217 passed, 1 failed. ZERO `error[` lines and 186 test targets in
both.** The +1 is this campaign's new witness; the failure is the same one in
both runs.
That failure is the environmental published-sheet floor
(`ambition_sprite_sheet`: *"780 sheet(s), below the floor of 800"*), which is a
claim about THIS CHECKOUT's publish output and carries its own ⛔ DO NOT LOWER
THE FLOOR. ⚠ STILL 780 ON 2026-09-18, on the same box and in a full
`./run_tests.sh` — so it is a persisting machine state rather than one run's
accident, and `scripts/check_published_sheets_are_present.py` still answers ALL
173 ROSTERED TARGETS PRESENT beside it, which is the two-populations split that
row already names (`find ... -name '*_spritesheet.ron' | wc -l` reads 752
files). ⚠ Recorded with the count of TARGETS because a red lane runs a
smaller population than a green one — 186 here against the 49-vs-79 binaries this
row measured when a lane failed early. ⇒ It is also one more non-reproducing
sample of OPEN 1's 2026-09-10 original, which is a non-negative, not a negative.

**OPEN 1 — intermittent arms that fail only in company. FOUR instances; THREE are
closed with measured causes and all three are the SAME SHAPE: a per-arm
measurement reading process-global state. The 2026-09-10 original is the one that
is not.**
`does_a_presence_probed_row_move_when_its_value_does::decaying_animation_timers_reproduce_across_every_resimulation`
failed once in a full lane at `041b07158` on its own third anti-vacuity
assertion: *"the probe took 1 distinct census(es) at the frames the audit
COMPARED — so either the subject held one value at every one of them or the
projection is constant… `resimulations > 0` does not imply this: the window is
long and the compared frames are few."*

⇒ **MEASURED, same commit and box:** 3 of 3 runs of that arm alone PASS; 4 of 4
arms of its file pass together; a second full lane came back 691/0/41. One
failure in two full runs of 731 arms, zero in seven targeted runs.
⇒ **FOUR MORE FULL `app_it` LANES, 2026-09-17, all `713 passed / 0 failed / 45
ignored`** (417.30 s, 417.12 s, 412.45 s, 505.69 s). ⚠ Only the first two are
attributable to `9df7991e1`: the loop was started there and the tree was edited
under it while runs 3 and 4 were still going, so those two are full-lane passes
at a tree this row cannot name. A non-reproduction is weak evidence either way,
so the distinction costs nothing to keep and would have mattered for a RED.
⇒ **AND THE SIX-DAY-OLD NAMED NEXT STEP IS NOW TAKEN: a full lane at
`--test-threads=1` came back 691 passed / 0 failed / 41 ignored in 1053.40 s**
(at `23a0d21a6`, same box). ⚠ **That is WEAK evidence and must not be read as a
verdict** — the arm passes most parallel runs too, so one clean serial run is
equally consistent with parallelism being irrelevant. The result that would have
been strong is the opposite one: a serial run that still failed would have ruled
parallel contention out in a single shot.

⛔ **CPU contention is REFUTED as the mechanism** — twelve busy-loop processes on
a 12-vCPU box, then the arm three times: 3 of 3 PASS. Wall-clock starvation is
not the variable. What remains that a 731-arm run has: many Bevy apps alive at
once, shared target-dir and asset I/O, libtest's thread scheduling — candidates,
not findings. ⚠ **Do not add a retry.** ⚠ And `measure` prints its census count
on every run while libtest swallows stdout for a PASS, so the diagnostic that
would show the window drifting needs `--nocapture`.

✔ **AND THE CLASS NOW HAS A RATCHET:
`scripts/a_test_static_is_a_channel_between_arms.py`.** `app_it` runs its arms as
threads of ONE process, so an interior-mutable `static` in test code is shared by
every arm that reaches it. The census is TEN, all ten adjudicated with reasons —
two serialising locks, one cross-arm filename sequence, two once-built immutable
casts, and five single-arm recorders marked LATENT because a second arm touching
one inherits the defect. ⚠ `thread_local!` is the remedy and is not reported; a
per-call closure is better still, because it is per-USE rather than per-thread.

⛔⛤ **AND THE FIRST VERSION REPORTED THE REMEDY AS THE DEFECT**: `thread_local!`
declares its cells with the `static` keyword one line down, so two of them read
exactly like a shared counter. ⚠ The arm holding that exclusion ALSO had to be
re-routed — it called the helper directly, so poisoning the production call site
left it green while only the repository ratchet noticed. A unit test of a helper
is not a test of the wiring that uses it.

✔ **AND A FOURTH INSTANCE IS CLOSED BY THE SAME MECHANISM, WHICH IS WHAT MAKES
IT A MECHANISM RATHER THAN A COINCIDENCE.** The triage page's own 2026-09-16
instance — building a second sim App made
`no_registered_type_is_written_outside_the_rewinding_schedule` report **99** types
written outside the rewinding schedule — was the same shared-counter channel:
`how_much_of_the_peer_checksum_actually_varies` drove every fixture from a
`playing()` cadence whose phase lived in a `static AtomicUsize`. Controlled:
**15 of 15 pass with a per-call cadence, 8 of 10 FAIL with the `static` restored.**
The arm that was `#[ignore]`d for it runs in the lane now. ⇒ Three of the four
instances now share one cause, and it is never the engine: it is a per-arm
measurement reading process-global state.

⛔⛤ **AND IT IS THE SECOND INSTANCE OF ONE SIGNATURE, SIX DAYS APART.**
[`triage/a-composition-acceptance-that-only-fails-in-company.md`](triage/a-composition-acceptance-that-only-fails-in-company.md)
recorded the same shape on 2026-09-10:
`composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`
FAILED inside a full `cargo test --workspace` run and PASSED alone, twice, and a
repeat of the same binary did not reproduce it. Different arm, different subject,
identical signature — **fails in company, passes alone, intermittent rather than
deterministic-in-company.**

⇒ **AND THE CLASS NOW HAS A MECHANISM, FOUND BY APPLYING THE THIRD INSTANCE'S
CAUSE TO THE FIRST: A PER-ARM MEASUREMENT READING PROCESS-GLOBAL STATE.** The
`hall_transition_cover` census classified images through a `static` ledger keyed by
a per-App asset id; this arm's INPUT came from a `static AtomicUsize`. `app_it`
runs its arms as threads of ONE process, so both were reading state a sibling
writes.

⛔⛤ **ROOT CAUSE, 2026-09-16.** `landing_repeatedly` — the cadence this arm feeds
the simulation — kept its step counter in a `static AtomicUsize`, because
`measure` took a bare `fn() -> AgentAction`, which cannot carry state. TWO arms
draw that cadence (this one and
`a_constant_projection_over_actors_folds_to_one_value_and_reports_nothing`), so in
company they interleave one counter and each receives an arbitrary subsequence of
the phases. `jump: n % 8 == 0` is how the body leaves the ground ⇒ an arm drawing
no multiple of 8 never lands, nothing arms `land_anim_timer`, and the subject holds
one value for the whole window — which is *precisely* the captured failure,
*"the probe took 1 distinct census(es) at the frames the audit COMPARED"*.

⭐ Fixed by making the cadence a per-call closure (`measure` takes `impl FnMut()`
now) and held by `two_arms_drawing_the_same_cadence_receive_the_same_input_sequence`,
which draws two sequences INTERLEAVED and floors the jump count on BOTH. Poisoned
by restoring the `static`: *"the second sequence drew 0 jump(s) in 120 steps"*.
⛔⛤ **The first version of that arm drew the sequences one after the other and
PASSED with the defect restored** — a shared counter offsets the second window by
`DECAY_STEPS`, which is 120, a whole number of the cadence's 8-step periods, so the
two came out identical. An arm satisfied by an offset that happens to be a multiple
of the period is measuring the arithmetic, not the sharing.

⚠ **WHAT IS STILL OPEN: the 2026-09-10 instance**
(`composes_through_the_sdk::a_host_that_omits_boss_encounters_still_builds_and_steps`),
whose assertion was never captured — **and the mechanism above does NOT explain
it.** Checked the same day the mechanism was found, three candidates and all three
refuted: the arm's own file holds no `static`, `ambition_platformer2d_runtime` does
not depend on `ambition_render` so the engine group cannot reach the image-stage
ledger, and the disabled plugin's crate holds exactly two globals — a warn-once
dedup `BTreeSet` and a read-only `LazyLock` catalog — neither of which can zero a
fixed-step count or panic a plugin build.

⭐ **THE MAP FOR WHOEVER LOOKS NEXT, MEASURED:** production code outside tests
holds roughly **fifty `static`s with interior mutability** (`Mutex`, `RwLock`,
`OnceLock`, `Atomic`, `LazyLock`), concentrated in `ambition_sprite_sheet` (12),
`ambition_characters` (7), `shared_tangle` (4) and `ambition_causal` (4). That is
the footprint of the mechanism in a binary that runs its arms as threads of one
process. ⇒ The next step for this instance is still its ASSERTION, not another
sweep: it has never been captured, and `step_the_fixed_schedule` has two ways to
fail that the exit code does not distinguish.

OPEN 2 below may be a fourth. ⚠ That
page has sat open, unattributed and LINKED FROM NOTHING for six days — it was one
of two orphans in the whole planning tree — so its named next step was never
taken. It says: *"a repeat run with `--test-threads=1` and a fixed seed order,
comparing against the failing composition — NOT a fix."* ⭐ And its sharpest
sentence generalises to tonight's arm exactly: *"its own doc says the only thing
that would make it fail is X. That sentence is a claim about the SUBJECT. This
failure is a claim about the HARNESS, and the two are indistinguishable from the
exit code."*

⛔⛤ **A THIRD INSTANCE, 2026-09-16 — AND THE FIRST ONE FIXED.**
`hall_transition_cover::two_round_trips_through_the_gallery_return_the_same_working_set`
failed in a full `app_it` run on *"the hub's resident character PAGES differ
between laps (70 → 71) — more is a retention, fewer is a page that never came
back"*, passed alone, and a second full run of the same binary came back
**706 passed / 0 failed / 46 ignored**. Same signature as the two above: fails in
company, passes alone, does not reproduce.

⭐⭐ **AND THIS ONE IS ROOT-CAUSED, BECAUSE THE ARM PRINTS ITS OWN WINDOW.** The
captured log, company against alone:

```text
company (FAILED)  lap 0: to hall in 233 frames: (258,  70, 302.59); back: (258,  70, 302.59)
                  lap 1: to hall in   4 frames: (258,  70, 302.59); back: (258,  71, 304.81)
alone   (3 of 3)  lap 0: to hall in 132 frames: (258, 149, 482.75); back: (258, 149, 482.75)
                  lap 1: to hall in   4 frames: (258, 149, 482.75); back: (258, 149, 482.75)
```

⛔⛤ **THE ASSERTION WAS NEVER ABOUT ONE RETAINED PAGE. THE CENSUS WAS LOSING
HALF THE WORLD.** Alone it reads 149 pages / 482.75 megapixels, three runs
byte-identical. In the failing lane it read 70, then 71 — so the *"one page too
many"* it reported was one of 79 absent pages arriving late. A first candidate
(`settle_resident_pages` going quiet over a still-growing page set) was measured
and DIED: instrumented across five settle calls the unbuilt count was 0 every
time.

⭐⭐ **THE CAUSE IS A PER-APP CENSUS ROUTED THROUGH A PROCESS-GLOBAL KEYED BY A
PER-APP ID.** `common::resident_character_pages` classified an image by
`image_stages::ledger().get(id)` — the ledger is a `static Mutex<ImageStageLedger>`
keyed by `UntypedAssetId`, `app_it` runs its arms as threads of ONE process, and
Bevy asset ids are per-arena indices that collide across Apps by construction. A
sibling inserting the same index with a different path OVERWRITES the row, and the
`row.path == path` branch written to keep siblings out then discarded THIS App's
page. Instrumented over one 6-arm run, counting why each image was rejected:

```text
no-path=22 no-row=0 other-source=76 PATH-MISMATCH=120 kept=29
no-path=22 no-row=8 other-source=70 PATH-MISMATCH=9   kept=138
```

⇒ One arm kept 29 of 149 and its neighbour kept 138, decided by whoever else was
running. ⚠ **The function's own doc already specified the fix** — *"the ledger is
consulted only as a CLASSIFIER: was this PATH demanded on the `character-sheet`
road"* — and the code keyed it on the id anyway. A path is a property of the ASSET,
not of the App that loaded it, so the classifier is a path set built from
`ledger.rows()` now. Measured: the 6-arm run goes **28 → 139** pages and the alone
run stays at 149, byte-identical. ⛔ No retry was added.

⭐ **AND IT MADE THE ORPHAN GUARD STRONGER.** `orphan_character_pages` subtracts
this App's owned sheet paths from this census, so a census that had shrunk to 29
was checking 29 candidates for leaks. It checks 139 now, and still passes.

⛔⛤ **AND THE PATH-KEYED CLASSIFIER FIXED THE VALUE WITHOUT FIXING THE
COMPARISON — SECOND INSTANCE, 2026-09-16, FOUND BY A FULL `cargo test
--workspace`.** Same arm, same shape: *"the hub's resident character PAGES differ
between laps (126 → 127)"*. The arm's own printed window settles it — 258
realizations on BOTH laps and megapixels up by exactly one page's worth — so the
App's character table did not move and the INSTRUMENT did. The classifier is a
process-global set that only GROWS, and a sibling arm demanding a new
character-sheet path between lap 0 and lap 1 starts counting a page this App
already held.

⚠ **AND THE OBVIOUS REPAIR WOULD HAVE HIDDEN THE DEFECT THE ARM EXISTS FOR.**
Freezing the classifier before lap 0 makes both readings one instrument — and a
page THIS App loads during lap 1 is exactly what puts its path in the ledger
late, so a start-of-arm snapshot excludes precisely the new residency being
hunted. ⇒ The split landed instead: residency is recorded UNCLASSIFIED at each
reading (`common::resident_image_paths`) and classification is applied ONCE after
both laps. A page classified in between lands in both sets and cancels; a page
that arrived in between does not. ⛔ No retry, and no floor was loosened.
Measured: the arm now reads **149** pages in a full `app_it` run, which is the
number three alone-runs read byte-identically before any of this. Poison-verified
— an extra classified page on lap 1 only reddens it at 149 → 150.

⚠ **THE NEXT CANDIDATE IN THIS CLASS, NAMED NOT FIXED.**
`hall_redecode_census.rs` reads the same process-global ledger directly, and its
second assertion is `re_decodes == 0` over a DELTA of a process-wide counter
(`ledger.re_decodes - before_redecodes`). A sibling arm re-decoding one path
inside this arm's window fails THIS arm, and the failure would read as a Hall
regression. Its `routed > 0` premise has the opposite exposure — contamination
makes a floor EASIER to pass, so that half is weak rather than flaky. ⛔ No
failure of this arm has been observed; it is a mechanism with the right shape and
no instance, which is exactly what the two unexplained instances above still
lack.

**OPEN 2 — one older non-reproducing session-root handoff failure** whose
assertion was never captured. It did not reproduce again across four full runs
tonight, and the two arms it would have to be — `the_shipped_app_never_holds_two_session_roots_across_a_handoff`
and `a_candidate_session_replaced_while_pending_is_discarded` — passed in every
one, checked BY NAME in the log rather than inferred from the total. Both already
assert their own premises, so a future failure carries its cause in its message.
⇒ The next step is NOT more runs.

**Acceptance:** the failing population is reproducible or explicitly classified,
and the production cause is fixed or the harness proves why the failure is not a
production invariant.
