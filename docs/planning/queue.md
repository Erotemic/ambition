# The queue — live execution order

This file is the **current executable engineering queue**. It is not a work log,
review transcript or archive. Git history owns completed investigations. Durable
design and measurements belong in the linked owner document. Product decisions
belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

A row remains here only while an engineer can act on it. When it closes, keep a
short receipt only where another open row depends on that fact; otherwise remove
it.

⚠ **OWNERSHIP AS OF 2026-09-16T14:45Z, recorded because two sessions ended inside
one day and a stale owner line is worse than none.** CalculexAmbition stood down;
YardratAmbition is the primary session for the rest of the day.
ToothbrushAmbition's session is no longer reachable, so **post-A10 demolition,
`docs/planning/consolidation/*`, the TEST-LANES row, AGENTS.md's target-bindmount
section and `scripts/measure_test_arm_rss.py` are UNOWNED** — they are not
finished, they are unattended. ⇒ Check a row's owner against who is actually
running before waiting on them.

⚠ **THIS FILE IS 3,345 LINES AGAINST THE 908 THE C10 CLEANUP LEFT ON
2026-09-14** — re-derive with `wc -l docs/planning/queue.md` and the per-campaign
mass with

```sh
awk '/^### /{if(n)printf "%s %s\n", c, n; n=$2; c=0} {c++} END{printf "%s %s\n", c, n}' \
  docs/planning/queue.md | sort -rn | head
```

because a number stated here without its command is the next stale copy: this
paragraph carried 2,519 for long enough that the file had grown by 732 lines
underneath it. Three agents worked it in one night and it more than doubled,
which is the point — **the growth is continuous and the compression is per-row,
so a single cleanup does not hold.** C10's regression rule is *"if a live
control-plane file starts accumulating closed case files again, delete/compress
the history IN PLACE"*, so this is a note to every owner rather than a complaint:
**only a row's owner can tell a receipt from live work.**

⭐ **THREE WORKED EXAMPLES OF THE SHAPE THE CONTRACT ASKS FOR, offered rather
than requested.** TEST-LANES went 193 → 66, grew back to 159 in one night of real
findings, and everything in it that was a RULE rather than open work moved to
[`docs/recipes/running-the-heavy-app-it-lane.md`](../recipes/running-the-heavy-app-it-lane.md).
HEADLESS-STEP-COUNT closed at 242 lines and is now 54, its mechanism, classifier
and repair moved to
[`docs/recipes/checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md).
ID-PEER went 702 → 543 by moving the peer input-payload contract to
[`engine/netcode.md`](engine/netcode.md#the-input-payload-two-peers-exchange) and
the schema-instrument exclusion to that page's
[`N3`](engine/netcode.md#n3--contentschema-negotiation), keeping a receipt with
its SHAs. ⇒ A closed row keeps its RECEIPT and its prohibitions; the reusable
half belongs on a page people read before they have the bug.

## P0 — architecture and correctness

### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16

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

**NEXT IMPLEMENTATION STEP.** None, and post-A10 demolition is no longer an
active lane either — it CLOSED on 2026-09-16 on both axes, with a mostly NEGATIVE
result that is worth keeping because it is what stops the next reader re-running
it: of every A10 symbol, ALL have live production callers, and of every `pub`
item in `transaction.rs` and `stage.rs`, exactly one had no caller outside those
two files. One dead mechanism was found and deleted (`CandidateState`,
`09629b060`); one accessor was narrowed. ⭐ A demolition that finds almost
nothing is a result about the CAMPAIGN — A10 replaced its mechanisms rather than
layering over them — and it is only worth that if the negative is recorded. ⛔ Not part of A10: peer-stable identity
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

### CUTSCENE-ROLLBACK-DECISION — two session-scoped cutscene values cross into simulation with no rollback decision

**Owner:** unclaimed for the QUEUE half. Found 2026-09-16 while measuring C03
step 3; NOT fixed here. ⛔ Item 1 below is blocked on `Q136`, not unowned.

✔⛤ **MEASURED 2026-09-16, AND THE TWO HALVES CAME OUT DIFFERENTLY.** This row
used to say *"REASONED, not measured — there is no failing arm yet"*.
`CutsceneAdvanceRequest` is now held by a failing-by-design witness;
`CutsceneTriggerQueue` is **benign by accident**, and the row's reasoning about
it named a real structural gap but the wrong consequence. Both are rewritten
below. The partition, RE-DERIVED 2026-09-17: of
`SessionScopedResources`' 30 members, 24 are rollback-registered, 2 call
`declare_rollback_derived_resource`, and **4 carry no rollback decision of any
kind** — the same four, so this row's subjects did not move. ⚠ It read 22 / 3 / 4
of 29 a day earlier: one member joined the bundle and `AuthoredOccurrences`
stopped being declared derived (`f15461f52`). Reading the four, source already answers two of them:

- `BossEncounterRegistry` — *"Authored boss data… Read-only at runtime"*, *"The
  registry is a read-only DATA CATALOG (profiles only)"*, populated once behind a
  `specs_loaded` latch from the App-local catalog. Authored content, identical on
  both peers. ⇒ **Correctly unregistered.**
- `CutsceneSkipHold` — *"The input-local half of the skip: an accumulator the HUD
  draws and the sim never reads"*, stated beside its own `init_resource`.
  ⇒ **Correctly unregistered.**

⛔⛔ **THE OTHER TWO ARE WRITTEN OR CONSUMED INSIDE THE REWINDING SCHEDULE.**

1. ⛔⛤ **`CutsceneAdvanceRequest` — CONFIRMED, AND A DISMISS PRESS DOES
   NOTHING.** It is produced by `apply_menu_frame_to_cutscene_request` on the
   HOST side in `Update` and consumed by `tick_active_cutscene` with
   `std::mem::take` inside the sim schedule's `Cutscene` phase. Held by
   `a_cutscene_dismiss_raised_outside_the_simulation_is_lost`
   (`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`),
   playing the shipped `cutscene_lab_intro` up to its `Dialogue` beat:

   ```text
   dismissed from INSIDE the sim schedule    beat 1 -> advances (to 3)
   dismissed from OUTSIDE it (the host)      beat 1 -> stays at 1
   ```

   ⇒ The first pass takes the edge and advances; the rewind restores
   `ActiveCutscene` (which IS `cutscene.playback`) but not the request, which is
   unregistered and already `false`. Nothing re-produces it. ⭐ The in-sim arm is
   the control: a fixture that cannot advance a cutscene at all prints the same
   stalled `1`. Poison-verified (0 → beat 3).

2. ✅⛤ **`CutsceneTriggerQueue` — NOT A LIVE DEFECT, AND THE REASON IS AN
   UNSTATED INVARIANT RATHER THAN A DECISION.** The structural description above
   was right: it is drained by `drain_cutscene_triggers`, which returns early
   without draining while a cutscene plays, so it holds across frames while
   `ActiveCutscene` and `LastCutsceneRoom` rewind around it. But the consequence
   does not follow, because **every producer is itself inside the sim schedule**
   — enumerated, not assumed: `auto_trigger_room_cutscenes`
   (`Platformer2dSimulationPhaseMonolith::Cutscene`) and
   `update_boss_encounters` → `publish_events` (`ProgressionSet::BossAdvance`).
   A replay therefore re-produces whatever the rewind dropped. Measured: one
   in-sim trigger on a single tick starts a cutscene that stays up for 171
   frames of resimulation.

   ⚠ **SO IT IS CORRECT BY COINCIDENCE, WHICH IS THE THING TO WRITE DOWN.** The
   moment any producer moves to `Update` — exactly where
   `apply_menu_frame_to_cutscene_request` already sits — it becomes item 1.

   ✔ **THE OWED INVARIANT IS STATED AND RATCHETED AS OF 2026-09-17, AND IT IS A
   SET RATHER THAN A SCHEDULE.** The sentence lives at the type
   (`CutsceneTriggerQueue`'s own doc) and the guard is
   `scripts/check_sim_consumed_request_writers.py`, in `--maintenance`: it
   enumerates every `ResMut<T>` / `&mut T` writer, requires each to carry a
   recorded reading of WHICH SCHEDULE it runs in, and fails on a writer nobody
   adjudicated or a recorded writer that no longer exists. **5 production writers
   today**, all adjudicated — two producers, one consumer, one helper the boss
   system calls, and the session-teardown bundle that CLEARS it at a different
   boundary. ⛔ **It deliberately does NOT attribute schedules**: a schedule's
   per-system access set is `pub(crate)` in Bevy 0.19 and `System::name()` is the
   debug placeholder in this build, so the guard makes the question unavoidable
   instead of answering it — and says so, because a check that looked like an
   attribution would be trusted as one. Poison-verified in both directions: a new
   production writer is named and fails, and a pattern that matches nothing trips
   the floor rather than passing over an empty set. ⇒ Item 2 no longer owes
   anything; it does not owe a fix today either.

⭐⭐ **AND THIS EXACT SHAPE IS ALREADY SOLVED ONE DOMAIN OVER, WHICH IS THE
STRONGEST ARGUMENT THAT IT IS REAL.** `OutstandingCheckpointRequest` is a request
raised outside the frame that spends it, and its registration carries the reason:
*"⛔⛔ A REQUEST THAT OUTLIVES ITS FRAME MUST REWIND WITH THE WORLD. The reset
channel is cleared on rollback, so before this bit existed a rewound timeline
simply lost the request."* Same sentence, same mechanism, different domain — and
the cutscene pair has no equivalent.

⚠ **AND `teardown.rs` ALREADY PAID FOR HALF OF IT AT A DIFFERENT BOUNDARY.** A
2026-09-13 review found that clearing `ActiveCutscene` and the trigger queue at
session teardown while leaving `CutsceneAdvanceRequest` let *"A's skip"* be spent
on *"B's scene"*. ⇒ The SESSION boundary was closed. The ROLLBACK boundary, which
is the same value crossing a different edge, was not looked at.

**What would settle it (not done here):** a SyncTest arm that raises
`skip_cutscene` on frame N, forces a rewind across N, and asserts the skip is
still spent exactly once — the same construction the checkpoint request's arms
use. A poison that removes the registration must redden it.

⛔ **DO NOT "FIX" THIS BY REGISTERING BOTH.** `CutsceneAdvanceRequest` may belong
on the control frame rather than in a resource — the per-seat frame-mode work
moved a policy the same way for the same reason — and that is a design question,
not a missing line. The decision is what is missing, not the registration.

⛔⛤ **AND THE DECISION IS ALREADY FILED, WHICH THIS ROW DID NOT SAY FOR A DAY —
CORRECTED 2026-09-17.** It is
[`Q136`](awaiting-maintainer-decision.md#q136--how-does-a-local-menu-intent-enter-the-synchronised-timeline),
*"how does a local menu intent enter the synchronised timeline?"*, and that
question already names `CutsceneAdvanceRequest` as **the residue of its own
census** — the one row of fifty-two cross-written resource types that uses none
of the four shipped stand-down patterns. ⇒ Item 1 is not unowned work waiting for
someone to notice it; it is BLOCKED on `Q136`, and the four precedents that
ruling enumerates are what a repair would choose between. A row that states a
missing decision without routing to the ledger that holds it leaves the decision
unasked.

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
sharp. The twelfth is `gravity.flip_switch`, and it is silent because NOTHING IN
PRODUCTION SPAWNS ITS COMPONENT — one construction site in the workspace, inside
a `#[cfg(test)]` module — which is `Q137`'s subject, not a gap in this walk.

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
⚠ And [Q136](awaiting-maintainer-decision.md#q136--how-does-a-local-menu-intent-enter-the-synchronised-timeline)
is the likelier first customer: if a cutscene edge is ruled to be gameplay input,
this is the arm that will speak.

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
| **the snapshot schema fingerprint** | ⛔ **OPEN, AND BLOCKED ON A MAINTAINER — `Q122`.** `schema_dump()` emits a prose `detail` per row and `compute_schema_fingerprint` hashes the whole dump, so English wording is inside the identity `ActiveRollbackAuthority::installed` gives a timeline. Measured by poison: pluralising ONE WORD in `detail::MESSAGE_CLEAR` turns the baseline red with 166 diff lines, 83 added and 83 removed. That is host-local lineage in a peer-stable identity in its purest form — two builds of the SAME mechanical schema are two identities if somebody reworded a comment. ⚠ The naive fix is refuted: of 493 rows, 268 carry facts `kind` does not encode (entity handle vs SET vs keyed MAP remapping, identical vs presence-aware canonical checksums, 22 custom-checksum descriptions), so dropping `detail` would stop the fingerprint seeing an entity-remapping change. The shape is a split, and where the line falls is the decision. ⇒ Landed meanwhile without needing it: the 15 sentences had TWO owners across two crates with nothing comparing them, and now have one (`879a5a1a3`, dump byte-identical). ⭐⭐ **AND A SECOND, 2026-09-17, WHICH THE DUMP COULD NOT SEE BECAUSE BOTH SPELLINGS WERE RECORDED IN IT:** five portal message types (`ClearPortals`, `DropPortalGun`, `FirePortalGun`, `PickUpPortalGun`, `TogglePortalGun`) each carried TWO rows — a canonical name and a historical alias kept *"so the full compatibility registration keeps the rollback schema byte-for-byte"* — and nothing outside the definition and the baseline ever read the alias. ⚠ It was not only a duplicated row: `should_install_backend` dedupes on the registration's stable NAME rather than its type, so each alias also installed a second `clear_message_channel::<T>` into `LoadWorld::Mapping` — ten systems doing five jobs, excused by name in `boot_budget.rs`'s deliberate-duplicate list. Removed; **493 rows → 488**, schema v196 → v197, the five exemptions deleted in the same commit because a stale excuse is a hole with a comment over it. ⚠ **488 IS THAT COMMIT'S NUMBER, NOT TODAY'S** — `Q142` added three rows on 2026-09-17 (v197 → v198) and the baseline holds **491**; re-derive with `tail -n +2 game/ambition_app/tests/rollback_schema_baseline.txt | wc -l` rather than reading either figure as current. ⇒ Held by `no_two_schema_rows_describe_the_same_type_the_same_way`, stated over (TYPE, KIND) because one type legitimately holds rows of DIFFERENT kinds and two rows of the same kind say one thing twice |
| **the accumulating gameplay clock** | ✔ **CLOSED 2026-09-16, FOUND BY THE TWO-HOST PEER-VISIBLE CENSUS.** `GameplayElapsed(f32)` has ONE writer — `advance_gameplay_elapsed`, `+= scaled_dt` every frame — is `init_resource`'d at App build, and was reset nowhere, while registered `rollback_resource_canonical` so its WHOLE value is compared between peers. Two hosts that reached the same route by different shell histories disagreed about it on the frame they arrived. ⭐ **UNLIKE `SimTick` IT NEEDED NO RULING, WHICH IS THE WHOLE DIFFERENCE**: `Q128` is open because a projection excluding the tick would exclude the TIMELINE, and this is a lookback clock whose only consumer is the brain's reaction-latency window (`actors/update.rs`), which a session-relative clock answers identically. ⇒ Added to `SessionScopedResources`, reset at `SessionScopeSet::Activate` — the group that already existed for exactly this. Held by `the_peer_visible_surface_does_not_record_which_route_the_host_visited_first` |
| **the startup-resume checksum** | ✔ **CLOSED 2026-09-16, SAME CENSUS.** `SessionStartupResume::checksum` — the projection handed to `rollback_resource_clone_checksum`, i.e. the function peers compare — hashed the session generation itself, and that generation is `SessionScopeId.0` (`restore_checkpoint_on_session_start`: `let generation = scope_id.map(\|id\| id.0)`). Measured `4354685564936845353` against `4354685564936845357`, scope `0` against scope `2`. ⇒ The projection now tags the generation's PRESENCE and drops its value. ⭐ **THAT IS ONLY SAFE BECAUSE THE ANTECEDENT IS ALREADY SHUT**, which is the `MatchInstance` lesson applied rather than repeated: excluding a local stamp with nothing identifying WHICH session the value describes is false-NEGATIVE, and `reset_checkpoint_coordinator_on_activation` already defaults this resource at the activation edge, so a foreign generation cannot be alive to compare. The generation STAYS in the value — `state_for` filters on it |
| **an ECS entity index in a peer checksum** | ✔ **CLOSED 2026-09-16, AND IT TOOK TWO CAUSES AND ONE REFUTATION TO GET THERE.** `PerceptionMemory` differed between two hosts whose only difference was which routes they visited first. ⛔⛤ `GameplayElapsed` was the obvious cause — perception is handed that clock and `RememberedActor::last_seen` is *"sim time the actor was last directly in view"* — and **fixing the clock MOVED the row without equalising it** (veteran `10957388069613372399` → `5036184031634534872` while fresh held). A mechanism that explains the number is not evidence for it. ⇒ The second cause was `collect_perception_peers`, whose id fell back to `format!("e{}", entity.index())` for a body with no `FeatureId`. That string is the KEY of `WorldMemory::actors`, a `BTreeMap` inside a `rollback_component_canonical` component — so a Bevy allocation-order artefact was compared between peers. **Measured: the same route perceives the player as `e888` in one host and `e1026` in the other.** ⚠ AND THE CHECKSUM IS THE SMALLER HALF: `WorldMemory`'s own doc says it is a `BTreeMap` because *"`last_known_hostile` takes the `max_by` confidence over these… so the tie is broken by iteration order"*, so two peers with index-built keys order their memory differently and an NPC with two equally-confident targets chases a different one on each. ⇒ The fallback is now `SimId`, which the body already carries; `FeatureId` stays first because that is what hostility and targeting look bodies up by. Held by the hostile arm, poison-verified in both directions. ⭐⭐ **AND DELETING THE FALLBACK FOUND A SECOND, OLDER DEFECT THE FALLBACK HAD BEEN COVERING — 2026-09-17.** The road refuses now (`error!`, `debug_assert!`, `continue`) rather than naming a body by its index, and the first full app run at that tree fired the refusal on the SHIPPED Ambition route in the only TWO arms that re-enter a live route — `id_peer_audit::two_hosts_with_different_local_history_seat_the_same_agreed_match` and `edit_to_play_through_the_shell::an_edit_on_disk_reaches_the_constructed_actor_through_the_shell` — one body each, `faction=Player primary=true player_entity=true scoped=SessionScopeId(1)`. ⚠ Both were a NEW red, not the intermittent this suite already knows about: the two failures carry the same panic at the same line, which is what said they had one cause. ⇒ The cause is an ORDERING window, not a missing mint: `ensure_sim_id` backfills `SimId::player_slot(0)` for a `PrimaryPlayer` body at the HEAD of the sim, so a player spawned into a re-entered route AFTER that system ran carried no canonical identity for the rest of its first frame — and `collect_perception_peers` reaches it in that frame. Before the refusal, perception simply remembered the player under its allocation index, which is exactly the artefact this row is about. ⚠ **A CENSUS BETWEEN FRAMES CANNOT SEE IT AND SAID SO:** the same fixture, inspected after every `step`, reports ZERO unnameable perceivable bodies at every step — the window opens and closes inside one frame, and only an instrument INSIDE the frame (the refusal itself) reports it. ⇒ Closed where the rule already said it belonged — *"its spawn site must mint an identity"*: `PlayerIdentityBundle` mints `SimId::player_slot(slot)` at construction, keyed on the bundle's OWN slot, so a second local player gets `slot:1` where the backfill would have given it `slot:0`. `ensure_sim_id`'s `PrimaryPlayer` arm is the net for a body that BECOMES primary later, and says so. The arm was RED at `2f05ec51f` and green after, which is the controlled pair, and the full app suite is **711 passed / 0 failed / 45 ignored** at the fix (it was 709/2 at the tree that found it). ⚠ **WHAT THE REFUSAL PINS IS NOW WIDER THAN THE PLAYER:** `ensure_sim_id` sweeps at the HEAD of the frame and again at the TAIL, and perception runs between them, so the assert is a standing claim that NOTHING reaches perception unnameable — mid-frame arrivals included. `UnmintedBodyCensus` cannot make that claim and does not try: it skips `Added<BodyKinematics>` this tick and treats `PrimaryPlayer` as nameable, both deliberately |
| **the 25 unchecksummed float rows** | ⛔ **OPEN, AND NOT ANSWERABLE IN THIS WORKSPACE.** Not a lineage road like the ten above — these carry no host-local id; they are simply never compared between peers. **S7** in [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md) ranks them: of the 99 rows outside the session checksum, 25 are also read by an unfiltered per-tick query AND carry a float-bearing field, and 12 of those are mutably written in production. ⚠ **THE COUNT OF WHAT IS MEASURED IS FURTHER DOWN THIS ROW, and this sentence used to carry a stale copy of it** (*"two are measured clean"*, while the same row said twelve of twelve fourteen lines later). What does not move: `Session::SyncTest` is the only session this workspace constructs, so a local resimulation clears a row of a RESTORE defect and says nothing about two peers. ⇒ The TIMELINE half's blocker is N2's absent P2P session, the same blocker `Q128` has. ⭐⭐ **THE STATE HALF IS NOT BLOCKED, AND SAYING IT WAS COST THIS ROW A ROAD — CORRECTED 2026-09-16.** No P2P SESSION can be built; two APPS WITH DIFFERENT LOCAL HISTORIES can. `two_local_histories_compute_the_same_mechanical_values` (`game/ambition_app/tests/shell_host_lifecycle.rs`) launches the shipped Ambition route first in one host and third in another (scope `0`/epoch `1` vs scope `2`/epoch `3`, asserted first), then compares `BodyAnimFacts` BITWISE by canonical `SimId` across 120 steps — and the ground items by construction, labelled as such because they are measured AT REST in that route. It agrees. ⚠ It does not retire N2: no transport, no input exchange, no interleaving, no rebase. ⛔ Its first version was VACUOUS — two `Platformer2dSimHarness` instances in one process both read `SessionScopeId(0)` at tick 1, because a fresh App is a fresh counter, so the differing history has to live inside ONE App that has been somewhere first. ⭐⭐ **AND THE STATE HALF NOW COVERS ALL TWELVE SHARP ROWS RATHER THAN TWO, WITH ELEVEN OF THEM ACTUALLY CARRYING STATE — MEASURED 2026-09-17.** `two_local_histories_agree_about_the_sharp_unchecksummed_rows` (same file) takes the COMPLEMENT of the peer-visible arm's join: it asserts each of S7's twelve mutably-written float rows does NOT feed the peer checksum (so the two arms cannot drift into reading the same surface), then compares the probe census of those rows across the two hosts at EVERY tick through 120, in each of FIVE ROOMS. **12 of 12 registered, 11 carried state and agree** — `actor.animation_facts`, `actor.render_size`, `entity.transform`, `feature.hazard`, `item.ground_item`, `player.blink_camera_state`, `portal.emission`, `portal.gun_pickup`, `portal.placed`, `portal.shot`, `boss.death_animation` — and that eleven is pinned by SET EQUALITY, so a row losing its carriers reddens rather than passing quietly. ⛔⛤ **THE SIX-TO-NINE MOVE WAS A ROOM AND A PRESS, NOT A FIXTURE.** This row read the six silent rows as *"a fact about the route"*; for two of them it was a fact about the ROOM. `portal_lab` authors fourteen `Portal` placements and `basement_hazards` three `DamageVolume`s, so pinning the start room with `StartRoomOverride` + `StartRoomMustResolve` covered `portal.placed` and `feature.hazard` with no input road and no simulation change. ⚠ The first aim missed: `basement_npcs`'s `HazardBlock` is a SURFACE, not a `PlacementSchema::Hazard` — `DamageVolume` is the identifier that becomes `feature.hazard`. ⭐⭐ **THE NINTH IS THE ONE THING A ROOM CANNOT AUTHOR — A PRESS.** A fourth walk starts in `portal_bridge` (player x=94, authored `PortalGunSpawn` x=180, 20px half-extent) and drives `axis_x: 1.0` with an attack EDGE every ten steps through the shipped `drive_control_frame`; the gun is in hand on step 20 and `portal.shot` has carriers on both hosts thereafter, agreeing. Both hosts get the identical script — a pure function of the step index — so they still differ in exactly one thing. ⛔ The anti-vacuity floor is on the INTERSECTION: poison-verified both ways — one row's value perturbed reddens it naming the row, and misspelling every row name trips the floor with *"the registry spells 0 of the 12"* instead of passing over nothing; the new coverage set and the strict start-room resolve are poison-verified too (dropping a room names the row it lost; a mistyped room id panics with the valid list instead of booting elsewhere). ⇒ **THE AUTHORED HALF OF THE REMAINING WORK IS SPENT, AND THE CHEAP DRIVEN HALF WITH IT.** Of the three still silent, `gravity.flip_switch` cannot be placed by ANY route — its only mutable writer is registered exactly once in the workspace and that registration is inside a `#[cfg(test)]` module, and the gravity plugin states in its own words that nothing spawns the component in-game, so S7 ranked a writer no production composition installs and the honest count of reachable sharp rows is ELEVEN — filed as **`Q137`** in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), because the unreachable half is a whole vertical (two rollback registrations inside the schema fingerprint, a per-tick view rebuild over an always-empty query, a render sync that rebuilds from it every frame) duplicating a mechanic the shipped encounter `Switch` already owns and authored content already uses. ⛔⛤ **`portal.emission` WAS FILED HERE AS A THIRD AND THAT WAS WRONG IN BOTH HALVES — CORRECTED THE SAME DAY.** It was read as wanting an aimed script (fire at a reachable wall, then walk into it). It wanted no gun: `portal_lab` — already the second walk — AUTHORS the aperture (`a_purple`, a ground-ground pair at x 254..346, 174px to the player's right), so driving that walk reaches it at tick 50. What hid it was the OBSERVATION: `PortalEmission` lives 0.18 s ≈ 11 ticks and the ladder sampled 0/1/30/120, whose widest gap is 90. The arm now samples every tick, and both halves are poison-verified separately. ⛔⛤ **AND `boss.death_animation` WAS THE SECOND OF THE PAIR, ON THE SAME MISTAKE.** It was filed as wanting a boss death, "the most expensive fixture of the set". A boss does not have to die: `BossDeathAnimation::default()` is inserted AT SPAWN (`actor_spawn/mod.rs:1141`), there are ELEVEN authored `BossSpawn` placements across nine rooms, and `basement_boss` is now the fifth walk. ⚠ Its carrier is constant `(1, 0)` at all 121 observation points, so what is compared is PRESENCE and identity rather than a varying float — a real boss death remains the stronger observation and the expensive fixture. ⇒ NO row is left that a route does not reach. The one that remains, `gravity.flip_switch`, is waiting on a PRODUCT RULING rather than on a fixture, and re-checked 2026-09-17 with the same insert-site lens that overturned the two rows beside it, the claim held: one construction site in the workspace, inside a `#[cfg(test)]` module opening thirty-three lines above it. | <!-- cite-ok: `PortalGunSpawn` is an authored LDtk entity identifier (`ldtk_entity_contract.json`), not a Rust definition; the Rust name is `PortalGunSpawnSpec` -->
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
`crates/ambition_platformer2d_actor_monolith/src/rollback_registration.rs:436`.
⭐ THE RULE: a component whose PRESENCE is read by a query filter is
AUTHORITATIVE even when its value is derived. The demotion population looks close
to zero and nothing should be demoted on a keyword match.


### SCHEMA-IDENTITY-OWNER — the rollback schema had two recordings — ✅ DONE 2026-09-16

**Owner:** peer schema identity (`scripts/check_absence_contracts.py`, netcode
[`N3`](engine/netcode.md)). Filed and closed in one pass; the measurement is on
N3 and is not repeated here.

**What was wrong.** `rollback_schema_baseline.txt` (the runtime dump, 493 rows)
and `rollback-schema-baseline.json` (a source scan, 423 names) both claimed the
schema, in different lanes. They disagreed about **73 rows, 21 of which feed the
peer checksum — 15% of the 144 rows peers actually compare.** Not drift: three
structural causes, of which only one was the root glob. 26 names are colon-form
and the scan's pattern requires a dot; 47 live under `game/`; and the marker the
scan follows is one of FOUR spellings of "this file registers rollback state".

⭐ **THE FIX WAS A COLLAPSE, NOT A WIDER REGEX** — the scanner's own comments
record that chase being lost three times. A source scan cannot own this fact,
because a registration is a runtime call and the spellings are not a closed set.
`stable_schema_names` is deleted; the runtime dump owns the names and the Rust
lane guards it byte-for-byte.

⭐ **WHAT THE BASELINE HOLDS INSTEAD IS THE ONE QUESTION THE TREE CANNOT ANSWER
ALONE.** A dump has no memory of its previous self, so "did the version move WHEN
the peer-visible set moved" needs a frozen prior — which is exactly why that
earns a baseline and a second copy of the name column did not.
`the-peer-visible-schema-may-not-move-without-the-version`, 144 rows at
`ggrs-rollback-schema-v194`.

⭐ **LANDED GREEN AGAINST HISTORY RATHER THAN IMPOSED ON IT.** 14 commits changed
the checksum-feeding set and all 14 moved the version; the 2 that held it each
added one row of a kind nothing hashes, which `ambition_mount`'s registration
already documents as deliberate — so the guard permits that road, and a version
nobody is forced to bump meaninglessly keeps meaning something.

⛔⛤ **THE SLICE DROPPED `detail` AT FIRST AND WAS BLIND TO 48 OF THE 144 ROWS** —
found by reading `Q122`'s own measurement rather than by any arm here. A
`resource-clone-custom-checksum` row's sentence says what its `fn(&T) -> u64`
covers, 22 of them say 22 different things, and narrowing a projection moves no
name, no kind and no type. The rule now keeps `detail` exactly where it
DISTINGUISHES rows of its kind and drops it where it does not — which is `Q122`'s
proposed split applied at the granularity the dump already has.

⚠ **THIS STILL IS NOT `Q122`.** `compute_schema_fingerprint` hashes the whole
dump, `detail` included, and that is untouched. The repository now answers the
prose question two opposite ways in two places, which is a second witness for the
ruling, not the ruling.

Also fixed in the pass: `encoded_types` had the same `crates/`-only root and was
blind to nine `SnapshotState` sites in `ambition_content` (129 → 137 types). That
widening is safe where the name census's was not — it matches a plain `impl`
beside the type, not a registration road. The poison that proves it: removing a
GAME-side impl now reddens the ratchet, and before the widening it changed
nothing at all.

**Arms:** six in `scripts/tests/test_absence_contracts.py`
(`test_a_checksum_feeding_row_that_lands_without_a_version_bump_is_caught`,
`test_the_same_row_with_the_version_moved_is_allowed`,
`test_a_row_feeding_no_checksum_may_land_without_a_version_bump`,
`test_the_checksum_feeding_kinds_are_read_from_the_source_not_a_list`,
`test_a_narrowed_projection_moves_the_slice_and_a_reworded_kind_does_not`,
`test_the_peer_visible_schema_ratchet_holds_against_the_live_tree`) plus
`the_shipped_app_registers_the_same_schema_as_the_sandbox` in
`game/ambition_app/tests/rollback_schema_baseline.rs`. Each poisoned from the
production side and each fired on its own arm.

⛔⛤ **AND THE BASELINE EVERY ONE OF THEM READS WAS RECORDED FROM THE SANDBOX, NOT
THE SHIPPED APP** — a question none of the five asked, because the sandbox arm
stays green precisely by never asking. Measured: `build_visible_app` and
`Platformer2dSimHarness` register byte-identical schemas, so the guarded identity
IS the shipped identity. That is now an arm rather than an assumption, with the
floor that two EMPTY registries are also byte-identical.

⚠ Two poisons in this pass printed green because the EDIT NEVER APPLIED — a
reword aimed at a uniform kind matched 0 rows, and an anchor shared with a
sibling test matched 2. Both were caught only by asserting the anchor count
before running, which every poison here now does.


### ROLLBACK-MUTATOR-POPULATION — the mutator guard sees a quarter of rollback state

**Owner:** rollback scheduling (`scripts/check_rollback_mutators_run_in_sim.py`).

**Current state:** the widening LANDED. Population 139 → 338 types, findings
8 → 12, resource half at `068bb6034` and the component half at `fa910c50d`. The
guard exits 0 again as of `c1ffa812d`. What is open is the `Transform` blind
spot; the two findings that owed an argument have it (the ✅ block below).
⚠ What `--list` prints today, 2026-09-18, so the widening's number is not read
off this page a month from now: **342 rollback types** (floor 300), **60 system
param bundles** (floor 55), **518 systems** taking one mutably. The 338 above is
the widening's own reference point and is left as history.

⛔ **THE EXIT CODE MEANS "NO NEW OFFENDER", NOT "CLEAN" — read this before
trusting a green.** `ACKNOWLEDGED` is a second table making the OPPOSITE claim to
`WAIVERS`: a waiver says this system's drift across a rewind does not matter and
carries the argument; an acknowledgement says the drift is REAL and names the row
that owes it. **NINE** are banked — six here and three to
MENU-RESET-MIDSESSION — and they print to stderr every run. Re-run 2026-09-18,
the whole bank, so this number is measured rather than carried:
`adopt_occurrence_checkpoint_from_save`, `complete_durable_restore`,
`compute_music_intent`, `portal_dev_toggle_system`,
`reconcile_roster_with_frozen_topology`, `sync_ldtk_level_set` here;
`grid_menu_action_activated`, `kaleidoscope_menu_action_activated` and
`track_versus_roster` there; over 518 mutating systems.

⛔⛤ **AND THE NINTH ARRIVED BY A WAIVER LOSING ITS ARGUMENT, NOT BY A NEW WRITE —
2026-09-18.** `track_versus_roster` was WAIVED, and the waiver said in its own
words that it *"rests entirely on the write preceding the timeline:
`maintain_local_session` starts GGRS only once a live primary player body exists,
and at route entry the roster is still `RosterSeating::Proposed` with no bodies
seated."* Read at the source: `maintain_local_session`
(`rollback_ggrs/src/local_session.rs:249`) opens with
`session_world_entity(world).is_some()`, and its three start gates are a SESSION
WORLD, `durable_hydration_is_pending` and `SessionSeatingSource::Pending`. **There
is no body condition anywhere in it.** ⇒ The premise was false, and
DURABLE-HORIZON-CHECKSUM had already recorded the same correction for a sibling
waiver two days earlier — *"the body is the later fact, not the shared one."*
⚠ What survives of the waiver is the single-write half, which is still measured:
one write at route entry, and in production only the route EXIT can arm it again.
What does not survive is *"and the timeline cannot have started yet"*, and
`VersusMatch` is `rollback_resource_clone_checksum`, so the write it is about is
peer-compared. ⇒ **A WAIVER IS A CLAIM ABOUT THE TREE AND ROTS LIKE ANY OTHER.**
This one was written when it was true of something and never re-read against the
system it names; the guard cannot check a prose premise, so the only defence is
re-reading the cited function when the row is touched.
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
SHORTER and CLEANER, which reads as good news. A fifth spelling cannot announce
itself, but it cannot avoid making the count fall.

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
2026-09-16 AND SEPARATES THE TWO.** A New Game's `NewGameResetCommitted` is
produced inside the rewind window, is `message-clear` in the schema
(`message.sandbox_reset_committed`, baseline line 378) and is consumed in
`Update` — broken whether or not the room replacement rebases GGRS.
`SessionScopeActivated` is a different shape at every end, measured:

| | `NewGameResetCommitted` | `SessionScopeActivated` |
|---|---|---|
| written by | inside the rewind window | `translate_shell_session_lifecycle`, literal `Update` (`crates/ambition_game_shell/src/session.rs:394`) |
| read by | `Update` | `reset_session_scoped_resources_on_activation`, literal `Update` (`crates/ambition_platformer2d_actor_monolith/src/session/teardown.rs:506`) |
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
and the two menu writers ([MENU-RESET-MIDSESSION](#menu-reset-midsession--the-menu-writes-rollback-state-from-update)).
Neither needed a new idea, only a population nobody had quietly narrowed.

**Acceptance:** the population is every rollback registration, not one
registration spelling; `handle_ldtk_hot_reload` is visible without its waiver
being deleted; a poison that respells a write in any supported param form still
reddens the guard; and the population floor fails when a spelling stops matching.

### DUP-SESSION-CURRENT — one owner per session-identity question

**Owner:** session lifecycle / shell-to-simulation boundary. The classification
ledger row is in
[`consolidation/architecture-census.md`](consolidation/architecture-census.md);
this is where the execution lives.

**Why it is here:** `consolidation/architecture-census.md` carried it as
`NEEDS_SEMANTIC_REVIEW` / `SOURCE_INFERRED` with the mitigation *"audit writers
and readers AFTER A10"*. A10 closed 2026-09-15, so the gate was discharged and
nothing said so. Jon named it the highest-value consolidation target after
ID-PEER; it was picked up opportunistically because two of its five members
(`ActiveSessionScope`, `SessionRoot`) are ID-PEER's own tokens.

**The audit, measured 2026-09-16.** Four types carried the
`(ShellActivationId, SessionScopeId)` correlation, which is the row's inferred
"several repeat scope/activation correlation" turned into a count:

| carrier | kind | verdict |
| --- | --- | --- |
| `GameplaySessionInstance` (in `ActiveGameplaySession`) | Resource | **THE OWNER.** One live session at a time |
| `GameplaySessionLinks` <!-- cite-ok: this row RECORDS the deleted type --> | Resource | **DELETED** — see below |
| `GameplaySessionWorldRoot` | Component | KEPT: a captured correlation on the entity it describes, which is what this row's own direction asks for |
| `GameplayInputOwner` | Component | KEPT, same shape |

⇒ **`GameplaySessionLinks` was a one-entry copy of a pair the owner already
carried.** <!-- cite-ok: named because it is GONE, along with its `scope_for` --> Three measurements, not one reading: activation asserts
`active_session.0.is_none()`, so its `Vec` could never hold more than one binding
and that binding was always the live session's; `scope_for` had **zero**
production readers <!-- cite-ok: `scope_for` is deleted --> (four test assertions, one of which literally asserted that
its answer equalled `ActiveGameplaySession`'s — the duplication written down as a
test); and the retirement block asked BOTH authorities in the same statement,
gating on the map and using the live instance's answer only for the load barrier.

⭐ **BEHAVIOUR-IDENTICAL, AND THAT IS A MEASUREMENT.** A probe asserting the two
answers agree ran the whole app suite green (709/0/45) BEFORE the deletion, and
`a_retirement_that_arrives_after_its_session_ended_changes_nothing` passes against
both the old code and the new. ⛔ I expected a repair here and there is none: the
suspicion was that the map gated the block more loosely than the live session
would, letting a delayed retirement reach the unconditional `GameMode` reset — the
2026-09-13 teardown shape. It cannot, because `unbind` <!-- cite-ok: the deleted method is what this sentence is about --> REMOVES the binding, so a
re-delivered retirement found nothing and skipped. The hazard needs an activation
bound and never retired, which the activation assert makes unreachable. ⇒ The arm
is there to hold the behaviour still while an authority is removed from under it,
not to witness a fix.

⛔ **WHAT MUST NOT COLLAPSE:** shell route identity and simulation identity stay
separate. `ActiveSessionScope` answers *"which scope is current, and is it
ready"* at the simulation layer; `ActiveGameplaySession` answers which shell
activation owns a session.

⛔⛤ **AND THE REASON THIS ROW GAVE FOR THAT WAS NOT MEASURABLE — CORRECTED
2026-09-16.** It said *"`ActiveGameplaySession` is `None` at launchers, credits
and every non-gameplay experience, while a scope can be live"*, and the second
half has no production witness: `ActiveSessionScope::publish` and
`active_session.0 = Some(..)` happen in the SAME run of
`translate_shell_session_lifecycle`, and the only other production caller
(`prepare_candidate_platformer_session`) RESERVES rather than publishes. Measured
directly: the engine-only harness composition reports
`ActiveSessionScope::current() == None`. A behavioural argument for a structural
rule was the wrong kind of argument.

⇒ **THE TWO REASONS THAT ARE CHECKABLE, AND ONE OF THEM IS NOW MECHANICAL.**
(1) A build-graph fact: `cargo tree -p ambition_platformer2d_shared_tangle -e
normal` reaches `ambition_game_shell` **zero** times, so the simulation layer
cannot name the shell's session even if someone wanted to — added to
`platformer-primitives-stays-a-foundation` in
`scripts/check_absence_contracts.py`, which checks the edge TRANSITIVELY and was
poison-verified by forbidding a crate the graph does reach. (2) The reserve/publish
split, which `a_hidden_candidate_session_is_invisible_to_the_live_world_and_visible_to_its_transaction`
exercises end to end inside `shared_tangle`'s own tests with no router in the
world: a candidate owns a scope identity that `current()` does not report, its
root and everything it owns are hidden from the live world and visible to its own
transaction, and admission promotes the POPULATION.

**Acceptance — all three clauses met 2026-09-16.** The correlation has one owner
and the two Components are projections of it; `ambition_game_shell` holds no
second map from activation to scope; and the layer split is a dependency contract
rather than a behavioural claim. ⚠ The third clause used to ask for *"a
composition with a live scope and no gameplay session"* — a witness that cannot
exist, which is why it went unwritten for as long as it did.

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
`rollback_coverage.rs` waives each one with that stated in its reason; this row is
where a reader looks first, and it was the copy that had drifted. ⚠ "Projected" is
not "admitted": a projection narrows who reads a mutable value, and the timeline
question is untouched by it.

**Blocked by:** the DAMAGE half only —
[Q127](awaiting-maintainer-decision.md#q127--are-difficulty-assist-and-player-damage-modifiers-match-wide-or-participant-specific).
⭐ **THE FRAME-MODE HALF WAS NOT BLOCKED ON A RULING AND HAD A RECORDED REPAIR**
(kept below because the repair it named was NOT the one taken, and the reasons
are the decision record).
The architecture review of 2026-09-13, quoted in `rollback_coverage.rs`: *"capture
resolves the semantic DIRECTION and simulation never sees a mode at all, at which
point this waiver and the row above both shrink."* That is implementation work,
not a decision — and it is strictly better than admitting the mode as state,
because it removes the concept from the simulation rather than versioning it.

⛔⛤ **AND THE RECORDED REPAIR HAS A COST NOBODY WROTE DOWN, MEASURED 2026-09-16
BEFORE STARTING IT.** *"Capture resolves the semantic DIRECTION and simulation
never sees a mode at all"* is implementable, and it is not free, because resolving
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
`drive_wave_encounters`; `GravityFlipSwitch` survives for the unit test).

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

⇒ Filed as [Q133](awaiting-maintainer-decision.md#q133--should-a-throw-obey-rage-when-obeying-it-changes-who-wins) — this is a balance call, not
a mechanics call, and it is not mine to make.

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

### HEADLESS-STEP-COUNT — ✅ CLOSED 2026-09-16: three arms whose green was wall-clock luck

**Owner:** CalculexAmbition. Measured and closed on the no-GPU box, which is the
honest place for it.

✅ **RECEIPT.** `add_headless_foundation` brings `MinimalPlugins`, which leaves
`TimeUpdateStrategy::Automatic`, so an `update()` steps the fixed schedule a
number of times derived from WALL TIME — measured, ten unpinned calls bought five
ticks with seven consecutive frames stepping none. Nine files were censused and
**all nine were read arm by arm; three arms in TWO files were real**, both in
`ambition_app`'s own headless path:

- `run_headless` / `run_shared_host_headless` reported `ticks_run: max_ticks` —
  the caller's own argument echoed back, so `assert_eq!(report.ticks_run, 8)`
  against `run_headless(8)` was `8 == 8`, in a test named
  `run_headless_runs_multiple_ticks`. **`6bab891e5`.**
- `sim_completes_60_ticks_with_counter_intact` ran 60 FRAMES while its comment
  said 60 ticks and asserted `last_frame <= total`, which is `0 <= 0` on a
  counter nothing had written. **`ba825d22a`.**
- `sim_accumulates_messages_across_repeated_attacks` asserted
  `BrainActionCounter::total >= 10`, and `total` counts every action by every
  actor. Poisoned by holding the button un-pressed for the whole run: **it still
  passed.** Now differential on MELEE messages, 10 pressed against 0 idle.
  **`ba825d22a`.**

**Fixed by** pinning `ManualDuration(timestep)` at each fixture and each runner,
counting real `FixedUpdate` executions, and looping until the TICK budget is met
rather than running N frames. **Guarded by**
`probe_how_many_fixed_steps_an_unpinned_headless_app_takes` (`013b70c89`) and by
each repaired arm's own poison. Closing measurements: `c6edd7e7c`.

⛔⛤ **AND THE REPAIR WAS OVERSTATED FOR ONE OF THE TWO RUNNERS — NAMED BY THE GPT
REVIEW OF 2026-09-16 AND NOW CLOSED.** *"Counting real `FixedUpdate` executions"*
is the right repair for `run_headless`, which is the direct sandbox host. It is
the WRONG CLOCK for `run_shared_host_headless`, which composes
`SimulationHost::Rollback` — the rollback backend advances `GgrsSchedule` from
`PreUpdate` via `RunGgrsSystems`, and **no invariant equates one Bevy fixed step
with one GGRS advance.** A rollback host may advance zero, one or several times
per outer frame depending on synchronisation and resimulation. So the field
honestly answered *"how many outer fixed steps ran"* and was named `ticks_run`,
which is what headless scripts read as *"the simulation ran"*.

⇒ **MEASURED, and the two numbers disagree in both directions:**

| run | outer fixed steps | simulation advances |
| --- | --- | --- |
| gameplay room, 30 ticks | 30 | **28** |
| launcher idle, startup budget | ~300 | **0** |

The launcher row is the review's own suggested poison, and this runner supplies
it free: no gameplay session exists, so nothing simulates however many outer
frames pass — while the old single number reported ~300 "ticks".

⇒ `SharedHostHeadlessReport` now carries `outer_fixed_steps` AND
`simulation_advances`, the latter counted by a system in `app.sim_schedule()` and
deliberately not rollback-registered so a resimulated frame counts again.
`a_gameplay_room_run_actually_advances_the_simulation` is the positive control
without which the launcher zero is uninterpretable — it was impossible to write
until the gameplay room stopped being an env var read inside the function body
and became an argument (`run_shared_host_headless_in_room`), because a parallel
test binary cannot safely mutate process environment. ⚠ The two counts are
deliberately NOT asserted equal: pinning a ratio would re-assert the invariant
the review showed does not exist.

⛔ **STANDING PROHIBITIONS THIS ROW BUYS.** Never assert about simulation state
after N `update()` calls without pinning the clock, and never report a count the
caller supplied. **The session world arrives on FRAMES with ZERO fixed steps** —
pinned to `ManualDuration(Duration::ZERO)`, `settle_until_session_world` still
returns `Ok(2)` — so "the app settled" is not evidence that anything simulated.

⇒ The mechanism, the classifier, the three-part repair and the census's own two
failure modes are in
[`docs/recipes/checks-that-did-not-run.md`](../recipes/checks-that-did-not-run.md),
under *"The clock nobody pinned"*. ⚠ The one line worth repeating here because it
redirects the next census: the risky population is **not** "tests that use the
engine foundation" — seven of the nine were clean, because demo fixtures already
insert `WorldTime { scaled_dt }` by hand and assert exact values — it is **code
that runs the PRODUCTION loop in a test process**, which is smaller and contained
every defect found.

⚠ **STILL OPEN, and deliberately not swept in:** the repair exists as a shared
helper, `step_the_fixed_schedule` in
`game/ambition_app/tests/composes_through_the_sdk.rs`, whose doc states the rule
this row needed — *"Pin the step, and then ASSERT THE STEP HAPPENED. The pin alone
is not enough."* No guard counts the population, so a tenth file can arrive
quietly. That is a cheap follow-up for whoever wants it and is not a defect today.

### DUEL-GUARD-RUNG — the CPU duel guard fails at rung 5 on main today

**Owner:** unowned. Found 2026-09-16 while measuring THROW-MODIFIERS; unrelated
to it.

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
both at the head of the sim. ⚠ **"BEFORE `CoreSimulation`" IS TRUE ONLY OF A BODY
THAT ALREADY EXISTS WHEN THAT SYSTEM RUNS**, corrected 2026-09-17: a body spawned
LATER in the frame waits until the next one, which is how the shipped player
reached `collect_perception_peers` unnameable (see ID-PEER's entity-index row).
The player's bundle mints its own identity now; the backfill is the net. The fixture carries them now, so those tests take
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

⛔⛔ **THIS IS NOT THE ONLY `A12`.** The frontier's
[`A12`](engine/actor-monolith-work-frontier.md#a12-align-flow-validation-prepared-representation-and-execution-bounds)
is *"align flow validation, prepared representation and execution bounds"* — a
different subject, and as of 2026-09-17 a CLOSED one. Both live on
`MovePlayback`, which is how one label came to cover two packets without anybody
noticing. ⇒ Name the subject, never the bare number.

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

⛔⛤ **READ THIS BEFORE ANY OTHER CPU NUMBER IN THIS REPOSITORY.** Measured
2026-09-16: `special_patent_clerk` throws ONE move for a whole 3600-tick bout at
rung 9 — `synchronize_clocks`×160, median gap 4 ticks, 0% damage either seat,
zero hitstun, Neutral 100% — and rung 9 is `RUNG_DEFAULT`, the rung every CPU
measurement in this project is taken at. ⇒ That is not a tuning observation. It
is a statement about what all the other numbers mean. Evidence and the full grid
below.

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

⇒ **AND IT IS NOT "LOW RUNGS ARE WORSE".** `goblin` locks at 5 and is fine at 9;
`special_patent_clerk` locks at 9 and is fine at 5. Both directions occur, which
is what `fighter_cognition_seed` mixing the character id with the level predicts:
each pair is its own deterministic stream, and some streams land in a basin the
decision layer cannot leave. Across the grid, 5 of 21 fighters fall under 40%
total damage at rung 5 and 1 of 21 at rung 9 — all five of the rung-5 ones are
healthy at rung 9 (`sanic` 0→130%, `goblin` 0→233%, `npc_pirate_admiral` 16→87%,
`npc_emmy_noether` 36→142%, `npc_carl_stargan` 28→116%).

⛔ **THE ONE THAT SHOULD WORRY A READER MOST IS AT RUNG 9, WHICH IS
`RUNG_DEFAULT`** — the rung every other CPU number in this project is taken at.
`special_patent_clerk` is locked there today.

⚠ **AND THE INSTRUMENT ALREADY NAMED THEM; NOBODY HAD RUN IT.** The sweep is
`#[ignore]`d as *"a measurement, not a guard"*, and its one assertion
(`silent.len() * 2 < ids.len()`) is deliberately about whether the TABLE is
readable, not about whether fighters fight — so one or two locked fighters print
their zeros and it passes. That is the documented design, not a defect in it.

**Next implementation for this half:** the lock is a decision-layer live-lock,
and `used == 1` with `neutral == 100%` over a 3600-tick bout is a crisp,
cheap predicate. ⇒ It is a candidate for a real guard, but ⛔ NOT by tightening
the sweep's existing assertion, which measures something else on purpose.

⚠ **BOUNDS THAT REMAIN.** Two rungs of the five, mirror matches only, one bout
per pair — and the pairs cannot be resampled, since the seed is
`(character, level)` with no clock. What is NOT bounded any more is the "one
character" caveat: 21 fighters, both directions of the effect.

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
   with a sentence naming what a frozen world breaks in it. Census 2026-09-17,
   last re-derived after the release-marker, room-cutscene and
   recharacterize-request arms landed: **29 fixtures, 15 reading a health API,
   12 adjudicated, 2 not arms** — re-derive
   with `python3 scripts/a_rollback_arm_must_refuse_a_frozen_world.py`, which
   prints the line. ⚠ It reads `git ls-files`, so a NEW fixture is invisible to
   it until staged; both of today's were caught only because the file was added
   before the sweep was believed.
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

### ROLLBACK-BAG-DESYNC — `AmbitionGameSave` disagrees with its own rollback replay

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
dialogue visit counter. Unhashing would make the repro green by throwing away
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
[ROLLBACK-BAG-DESYNC](#rollback-bag-desync--ambitiongamesave-disagrees-with-its-own-rollback-replay).

⛔ **WHAT IS LEFT IS NOT A SAVE WRITER AT ALL.** It is the DURABLE RESTORE
CHAIN's placement against GGRS start —
[Q135](awaiting-maintainer-decision.md#q135--should-ggrs-start-before-the-durable-restore-has-finished)'s
lifecycle half, characterised below and measured, not assumed. The ownership
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
| `adopt_occurrence_checkpoint_from_save` | `CustodyBaseline`, `OccurrenceBaseline` | **yes** | `Update` — ⛔ open |
| `restore_inventory_from_save` | `OwnedItems` + both item baselines | no | `Update` — ⛔ open, partial waiver below |
| `complete_durable_restore` | `SaveRestored` | no | `Update` — ⛔ open |
| the three `persist_*_to_save` | `AmbitionGameSave` | **yes** | ✅ sim schedule |
| `count_the_dialogue_visit_when_a_conversation_opens` | `AmbitionGameSave` | **yes** | ✅ sim schedule |
| `reset_inventory_on_new_game` + `reset_occurrence_horizon_on_new_game` | the reset baselines | no | ✅ sim schedule |

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
site — `durable_horizon.rs:504`, inside
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
Nothing replays it: the crate `ambition_dialog` contains the string `rollback`
**zero times**, `DialogState` is a plain `#[derive(Resource)]` with no
registration on any road, and the dispatcher consumed the request with
`state.pending_start.take()` in `Update`. ⇒ On a rewind across the start frame
`AmbitionGameSave` was restored to its pre-increment value, the request that
caused the increment was already gone and did not come back, and nothing re-ran
the dispatcher. **The visit was LOST, full stop** — one outcome, not two.

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

**THE RESTORE CHAIN — the half that is still open.**

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
pair's ordering against GGRS start is characterised rather than assumed; and Q135
is answered so the restore chain's three `Update` residents have a road. ⚠ The
guard stays banked-but-owed on all of these — see
[ROLLBACK-MUTATOR-POPULATION](#rollback-mutator-population--the-mutator-guard-sees-a-quarter-of-rollback-state)
— which is correct, and is why they were not waived to make a count go down.

### MENU-RESET-MIDSESSION — the menu writes rollback state from `Update`

**Owner:** `game/ambition_app/src/menu` + `ambition_platformer2d_actor_monolith`.

**Current state:** `grid_menu_action_activated` and
`kaleidoscope_menu_action_activated` write rollback-registered state from
`Update`, which does not rewind — `NewGameResetRequested` via
`SystemMenuParams::request_reset`, and `OwnedItems` via `dispatch_item_confirm` →
`apply_menu_action`, which spells the write `owned.take(Item::HealthCell, 1)`.
Filed off a harness that demonstrated it; `check_rollback_mutators_run_in_sim.py`
independently names both from source.

⛔⛤ **AND A THIRD ARRIVED 2026-09-18, FROM A WAIVER RATHER THAN FROM A NEW
WRITE: `track_versus_roster`.** It writes `*match_state =
VersusMatch::opening()` from top-level `Update` — deliberately outside
`GameplaySimulationRoot`, so route teardown survives leaving gameplay — and
`VersusMatch` is `rollback_resource_clone_checksum`, peer-compared. It was
waived on the argument that GGRS cannot have started yet *"only once a live
primary player body exists"*; `maintain_local_session` has no body condition at
all (session world, `durable_hydration_is_pending`, `SessionSeatingSource::
Pending`). ⇒ It is the SAME class as the two menu writers and it is banked to
this row. ⚠ It differs from them in one way that matters to the acceptance
below: theirs is a per-press write and this is ONE write at route entry, so the
window is narrower and the instrument has to sample the frame the
`(on_versus, mine) == (true, false)` arm fires — the shape
`probe_when_the_durable_restore_latch_flips_against_ggrs_start` uses, not the
240-frame harness the other two took. The full correction is in
[ROLLBACK-MUTATOR-POPULATION](#rollback-mutator-population--the-mutator-guard-sees-a-quarter-of-rollback-state).

⛔ **MEASURED, WITH THE CONTROL THAT MAKES IT READABLE.** An item granted from
outside the rewinding schedule is **GONE AT FRAME 0** under
`with_sync_test_rollback_settings(4, 10)` and **KEPT for 240 frames** in the same
world with no rollback session. ⇒ "The bag lost an item" and "the REWIND took the
item back" are indistinguishable from inside one harness; the control is the
load-bearing half of the arm.

⛔⛤ **AND THE RESET HALF IS NOW MEASURED TOO, 2026-09-16: A NEW GAME ASKED FOR
FROM OUTSIDE THE SIMULATION COMMITS ZERO TIMES.** Held by
`a_new_game_asked_for_from_outside_the_simulation_is_swallowed`
(`game/ambition_app/tests/a_bag_changed_mid_window_reaches_the_save.rs`), which
counts `NewGameResetCommitted` through one recorder on both roads:

```text
asked from INSIDE the sim schedule    1 commit (tick 31)
asked from OUTSIDE it (the menu)      0 commits, and the flag reads false again
```

⇒ **Pressing New Game does nothing under a rollback host, and leaves no trace.**
`NewGameResetRequested` is `resource-canonical` — snapshotted, restored, peer
checksummed — so the restore returns the flag to `false` before
`process_new_game_reset_request` (which runs INSIDE the rewinding schedule) ever
sees it.

⭐ THE IN-SIM ARM IS THE CONTROL AND IT IS WHAT MAKES THE ZERO READABLE.
`process_new_game_reset_request` has several *"DECLINE, do not die"* roads and
clears the flag BEFORE them, so "0 commits, flag false" is exactly what a
declining reset prints as well. The two arms differ only in WHERE the request is
written, so a decline would take both to zero. Poison-verified: registering the
in-sim requester in the outside arm takes it 0 → 1.

⚠ THIS IS THE WORSE HALF OF THE ROW. The `OwnedItems` defect is *"occasionally,
only in netplay"*; this one is deterministic — the request is written outside the
timeline every time, so the press is swallowed every time a rollback host is
running.

⛔⛤ **AND THE OBVIOUS REPAIR DOES NOT WORK, MEASURED BEFORE BUILDING IT.** Two
reviews named `ItemGrantRequested`/`ShopTransactionRequested` as the road a menu
action should take instead of writing rollback state. Those messages are
`clear_message_on_rollback` (`crates/ambition_items/src/rollback_registration.rs`),
so **the queue is rollback state too**, and their shipped producer is a
conversation node running INSIDE the sim schedule. Held by
`a_rollback_cleared_message_written_from_outside_the_simulation_is_also_lost`:

```text
one HealthCell grant, written from INSIDE the sim    bag 3 -> 4
the same grant, written from OUTSIDE it              bag 3 -> 3, never rises
```

⇒ **Re-spelling the menu's write as one of these messages would change nothing.**
A rollback-cleared message produced outside the timeline is exactly as loseable
as the resource write it would replace: the rewind clears the queue and restores
the bag, and nothing re-produces the request. The arm samples every frame, so
*"never landed"* is distinguished from *"landed and was reverted"* — it is the
former.

⭐ **SO THE QUESTION IS NOT "WHICH MESSAGE" BUT "HOW DOES A LOCAL INTENT ENTER A
SYNCHRONISED TIMELINE AT ALL", AND IT IS A PEER-VISIBLE DECISION.** A menu press
is a local input event; in rollback netcode local inputs reach the timeline
through the INPUT payload GGRS carries, not through a resource or a rewinding
queue. Putting a New Game bit there changes the input wire format, which two
peers must agree on. ⇒ Filed as [Q136](awaiting-maintainer-decision.md#q136--how-does-a-local-menu-intent-enter-the-synchronised-timeline) rather
than resolved by a quiet refactor. ⚠ Both witnesses are green ASSERTING THE
DEFECT and must be inverted by whatever lands; neither should be satisfied by
removing a type from the peer checksum, because
`install_resource_clone_checksum` installs the restore independently of the
checksum.

⛔ **AND NOTHING ANYWHERE SAYS SO.** No desync, no error, no log line.
`OwnedItems` is `rollback_resource_clone` — restored, not hashed — so there is no
checksum to disagree. `session_health` was clean on all 240 frames in which the
grant was being taken back, which also answers the obvious hope: the hashed
`OwnedItemsBaseline` does NOT stand in for the unhashed value.
⚠ And the save mirror never saw it either — `persist_inventory_to_save` is
value-compared and the restore lands before it next runs, so the autosave is
CONSISTENT with a world in which the equip never happened. That is why
`rollback_full_reset.rs` and `rollback_lifecycle_reset.rs` are green while
driving the same mirror: in those worlds the bag never changes.

⚠ **NOT A GGRS BUG, AND INVISIBLE IN SINGLE-PLAYER** — which between them is why
it survived. Restoring a snapshotted resource is what a rewind is FOR; the defect
is that a player-visible ACTION is expressed as a direct write to rollback state
from outside the rewinding schedule. With no session there is nothing to rewind,
so every hour of single-player play is evidence of nothing here.

**Two predictions of mine, both corrected by measurement, kept because they are
the ones a reader would reach for.**
⛔ **Item duplication is NOT the symptom.** It needs the HEAL to survive while the
ITEM returns, and it does not: `apply_menu_action` writes `PlayerHealRequested`,
registered `clear_message_on_rollback`, so both halves are undone together. The
real symptom is the quieter one — the menu action silently does nothing,
occasionally, only in netplay, with state self-consistent throughout. Harder to
notice and much harder to report, which argues for fixing it.
⛔ **The HASHED type is not the louder one.** This row first said
`NewGameResetRequested` would produce a DETECTED desync because it feeds the peer
checksum. Measured, it behaves exactly like the unhashed one: the write is erased
before it reaches a snapshot anyone compares. ⇒ **A checksum cannot disagree about
a value that was put back before it was taken.** Registration kind predicts
whether a SURVIVING divergence is caught; it says nothing about a write that does
not survive.

⚠⚠ **WHAT THESE ARMS CANNOT SHOW, so the row must not claim it.** A sync test is
ONE peer replaying itself, and a write erased identically on every replay
produces no mismatch. The LOCAL LOSS is measured for both types; whether TWO
peers would disagree in the window before the erase is NOT, and an earlier
version of this row asserted it. ⇒ The fix does not wait on that answer: a local
action that vanishes some of the time is already a defect.

⭐⭐ **THE FIX IS NOT A NEW PATTERN — `OwnedItems` ALREADY HAS A ROLLBACK-CORRECT
WRITE ROAD AND THE MENU DOES NOT USE IT.** `ItemGrantRequested` is
`clear_message_on_rollback` and its consumer `apply_item_grants` mutates
`OwnedItems` from the SIM schedule, beside `apply_shop_transactions`. So a
conversation that gives you an item is rollback-correct today and the MENU giving
you one is not, for the same resource in the same crate. The consuming direction
has a road too — `ShopTransactionRequested` with `ShopSide::Sell` — whose own doc
states the rule in the engine's words: *"a simulation system applies it on the
tick it was stamped for — every replay of that tick included."*

⛔ **AND IT IS NOT A PURE REFACTOR, WHICH IS WHY THIS IS FILED RATHER THAN DONE.**
The menu READS `OwnedItems` in the same frame to render the row it just changed.
A deferred write means the sim applies the grant on the next tick, so the list
shows the old bag for one frame unless the UI renders optimistically. That is a
visible behaviour change in shipped UI and a maintainer's call, not a mechanical
substitution — **`Q140`** in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), which puts
it as accept the one-frame stale row or render optimistically. A consumable USE is also not a shop sell, so the menu still needs
its own message — what it does not need is a new pattern.

⛔ **THE ESCAPE HATCH IS SHUT: the run condition GUARANTEES the dangerous window
rather than excluding it.** The hope is that such a menu cannot be open while a
session is live. These systems carry `.run_if(simulation_authorized)`, which
returns `live_scope_of(..).is_some()` — TRUE exactly when a live session scope
exists. ⚠ A live scope is not by itself a live GGRS session, but nothing here
narrows them to single-player.

⚠ Five sibling menu systems are WAIVED, not fixed: they take the same
`SystemMenuParams` bundle and never reach `request_reset`. If the bundle is split
so access matches use, drop those five waivers — they exist only because it
over-grants.

**Acceptance:** both writes go through a message the sim consumes; the repro arm,
which currently ASSERTS THE DEFECT so the lane stays green, goes RED and is
deleted with this row.

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
THE FLOOR. ⚠ Recorded with the count of TARGETS because a red lane runs a
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
