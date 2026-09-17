# Planning status

This page is a short orientation snapshot. Live execution is in
[`queue.md`](queue.md). Product rulings are in
[`maintainer-decisions.md`](maintainer-decisions.md); unresolved choices are in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). Durable
architecture belongs in the focused owner documents.

Do not copy detailed packet history into this file. Git history is the receipt for
completed work.

## Current architecture posture

Ambition is converging on **one mechanical owner plus explicit projections**.
Bevy remains the normal implementation substrate. Custom engine mechanisms are
reserved for semantics Bevy does not define: rollback authority, deterministic
identity, content generations, transactional construction/publication, session
ownership and persistence/custody.

The current architecture map and consolidation ledger are under
[`consolidation/`](consolidation/README.md). They are the cross-cutting reference
for authority/lifetime duplication; focused owner documents remain authoritative
for implementation details.

### Candidate world / last-good-world

**A10 is CLOSED (2026-09-15).** Room and session state are candidate-owned:
validation precedes one publication switch at both scopes, and outgoing state
retires only afterward. A verified publication also freezes the effects it owes
the world OUTSIDE its own population, so a room published inside a pending
candidate session announces nothing to the live one until that session is
admitted.

**And post-A10 demolition is CLOSED too (2026-09-16), on both axes.** Its result
was almost entirely negative: every A10 symbol has live production callers, and
exactly one `pub` item across `transaction.rs` and `stage.rs` had no caller
outside those two files. One dead mechanism was deleted, one accessor narrowed.
⇒ **There is no active A10 lane.** Peer-stable identity (ID-PEER) runs as a
separate campaign with a separate agent — see below.

⭐ **AND WHAT THAT UNBLOCKS, as of 2026-09-16: C03 and C05 are STARTABLE — every
gate on both is discharged.** The two that stood this morning are gone: the
shell/content A-supersedes-B race is witnessed in the shipped composition on both
halves, and the peer-identity checkpoint was discharged by its owner. ⚠ The
statement and its evidence live ONCE each — the witness in
[`consolidation/README.md`](consolidation/README.md), the checkpoint in ID-PEER's
queue row — and `consolidation-plan.md` points at them rather than restating
them, because this file has been the second copy that rots before. ⛔ One re-arm
condition is named there and not repeated here.

⚠ **STARTABLE IS NOT UNBLOCKED-ALL-THE-WAY, and the distinction is one measured
day old.** C03's three advertised cheap wins were measured on 2026-09-16 and two
are EMPTY — no session-scoped resource is reset-only, and the two reset lists are
a disjoint partition rather than two copies. The third turned into a maintainer
ruling: `Q132` asks whether a handoff frame holding two session roots should make
185 `SessionWorldRef`/`SessionWorldMut` sites run or skip. ⇒ **C03 can start; it
should not MOVE STORAGE before Q132 is answered**, because that ruling decides
whether a two-root frame may exist at all.

The row is [A10 in the queue](queue.md#a10--candidate-world--last-good-world-publication---done-demolition-closed-2026-09-16).

### Deterministic identity

Local lifetime/correlation identity and peer-stable mechanical identity remain a
separate active seam. `SessionScopeId`, shell activation ids, content epochs and
monotonic counters stay valid for cleanup and stale-message rejection; none of
them may determine authoritative RNG, deterministic construction provenance,
rollback identity, contact/projectile identity, or a peer checksum.

**The per-road table, the arm that holds each closed road, and every measurement
behind them are in
[ID-PEER](queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity).**
⛔ This section deliberately carries no road count and no per-road narrative. It
held both, and the corrections kept arriving here as separate hand edits AFTER
the owner row was already right: `dada0a4c5` and `28e087f43` each re-derived its
counts, and `86e8f9622` is titled *"the warning that a summary rots did not stop
me rotting the line under it"*. The last copy to go was worse than stale — it
said `SeatControlFrameModes` is *"read by sim systems"* while the owner had
recorded it closed and production holds exactly one `ResMut` writer and no
readers. A summary of a live table is a copy that corrections do not reach.

**What a reader picking this up needs: all three open roads are blocked outside
the campaign.** The fourth was open here on 2026-09-16 and closed the same day —
see the ⭐ note below the table for what it cost to close.

| open road | blocked on |
| --- | --- |
| the snapshot schema fingerprint hashing English prose | [Q122](awaiting-maintainer-decision.md#q122--which-registry-fields-are-mechanical-and-which-are-presentation) — where the mechanical/presentation line falls. The naive fix is refuted in the row |
| the 25 unchecksummed float rows | netcode **N2** for the whole class — they carry no host-local id and are simply never compared, which no projection fixes. ⭐ The STATE half is not blocked: a two-host differing-history arm measures one of them today (see the row) |
| the canonical timeline itself (absolute `SimTick`) | [Q128](awaiting-maintainer-decision.md#q128--should-the-simulation-tick-be-rebased-when-peers-agree-to-start-or-stay-an-absolute-per-app-count) — a projection excluding the tick would exclude the TIMELINE |

⚠ **NO SESSION IN THIS REPOSITORY CAN OBSERVE ANY OF IT** — `SyncTestSession` is
the only one constructed, one machine rewinding itself, zero distance, so a desync
canary compares a machine against its own past. ⭐ **A TEST CAN, THOUGH, AND THAT
IS NEW.** `the_peer_visible_surface_does_not_record_which_route_the_host_visited_first`
builds two hosts with different route histories, asks the registry which 145 of its
493 registrations feed the peer checksum, and compares exactly those. It found three
defects on its first run — after a JOIN that narrowed 364 probes to 145, because
the probe collection does not know which rows peers compare and over-reported
without it. ⭐ **The third cost two causes and one refutation.** Fixing the
accumulating gameplay clock MOVED `PerceptionMemory`'s value without equalising
it; the remaining cause was a perceived-actor id falling back to
`format!("e{}", entity.index())`, which is the KEY of a `BTreeMap` inside a
checksummed component — and whose iteration order also decides which of two
equally-confident hostiles an NPC chases. A mechanism that explains the number is
not evidence for it. That is the standing reason this seam gets review attention out of
proportion to what any test can currently fail on.

### Rollback-safe mechanical editing

Live mechanical editors use an explicit proposal/admission/publication boundary
rather than writing simulation authority directly. The census records six current
editor domains. Remaining work should extend that protocol when a new mechanical
editable domain appears; it should not create a second editor-specific rollback
road.

Mutable user preferences are distinct from admitted mechanical policy. Direct
`UserSettings` reads have been removed from the simulation schedule; the remaining
damage-policy lifetime is a product decision in
[Q127](awaiting-maintainer-decision.md#q127--are-difficulty-assist-and-player-damage-modifiers-match-wide-or-participant-specific).

### Persistence and the peer contract

✅ **REPAIRED 2026-09-16 (P0). THE SAVE MIRRORS NOW CROSS THE ROLLBACK
BOUNDARY.** `persist_inventory_to_save`, `persist_occurrence_horizon_to_save` and
`persist_minted_item_horizon_to_save` register through `app.sim_schedule()`, so a
rewind replays them. No disk I/O moved — they derive the save RESOURCE from live
simulation state, and autosave and the file write stay outside the simulation.

⛔ **WHAT WAS WRONG:** `AmbitionGameSave` is `rollback_resource_clone_checksum`,
so its whole value is compared per TICK, while the mirrors wrote it from `Update`
— per FRAME — and a rewind re-simulates ticks without re-running `Update`. A bag
that changed every tick desynced a GGRS sync test within six ticks; of 364 probed
rollback entries exactly one diverged, and it was the save.

⭐⭐ **AND THE ACCEPTANCE WAS NOT THE REPRO.** The merged-state review refused
*"startup repro now passes"* on its own, because a checksum that stops responding
to the state it covers turns every equality arm green in the direction that looks
like success. ⇒ Measured both ways: the divergence set went to EMPTY, **and** the
save's hashed projection went from **1 distinct census across 236 compared frames
to 236** (neighbours: 238). It is no longer PINNED, so the empty set is a
comparison that could have failed. The second measurement is guarded at a floor
of 50 so it cannot silently return to the pinned regime.

⚠ **STILL OPEN AND NOW THE ONLY HASHED-SAVE WRITER OUTSIDE THE REWIND WINDOW:**
`dispatch_pending_dialog_requests` INCREMENTS a dialog visit count from `Update`.
The mirrors could move because they DERIVE the save; an increment does not
converge under replay, so it needs a different answer.
⚠ **WHAT MAKES IT REPRODUCE IS THE FIRST THREE TICKS, NOT THE CADENCE** —
corrected 2026-09-16 by its owner after a sweep: an every-tick grant STARTING
at tick 4 runs 120 steps clean, starting at tick 1 or 2 it desyncs at frames
`[2, 3, 4]`, and N consecutive grants from tick 20 are clean at every N. This
page said *"a single change does not reproduce it"*, which was the second of
three framings. ⇒ The measurement and the correction live ONCE, in
[ROLLBACK-BAG-DESYNC](queue.md#rollback-bag-desync--ambitiongamesave-disagrees-with-its-own-rollback-replay);
do not re-derive them here.

⚠ This is not only a persistence question: 13 of the 19 systems that write
`AmbitionGameSave` are registered in the SIM schedule, so the save is
simulation-adjacent state in practice whatever it is in principle. ⛔ That is what
made "take it out of the checksum" the LARGE option rather than the small one,
and the review refused it outright — unhashing would have bought a green repro by
discarding comparison coverage for quests, flags, switches, encounters, shrines,
cutscenes and boss state.

⇒ **Q129 IS NARROWER FOR THE REPAIR AND STILL OPEN.** The desync is no longer
the reason to answer it, and its pinned-projection half is ANSWERED for the save
— it was pinned BECAUSE the mirrors wrote from `Update`. What remains is the
ownership question on its own merits: should a save FILE be part of what two
peers agree on. The ruling is
[Q129](awaiting-maintainer-decision.md#q129--must-the-save-file-be-part-of-what-two-peers-agree-on);
the measurement and the reproduction are in
[ROLLBACK-BAG-DESYNC](queue.md#rollback-bag-desync--ambitiongamesave-disagrees-with-its-own-rollback-replay).

⇒ Found beside it, and filed as
[Q130](awaiting-maintainer-decision.md#q130--should-the-sim-harness-refuse-to-step-an-invalidated-rollback-session):
an invalidated GGRS session keeps accepting `sim.step()` and stops advancing
`SimTick` in silence. Every rollback arm in the tree refuses a frozen world
today, none of them because a guard made it do so.

### Content generations and fast iteration

The host can consume edited move content without a Cargo/link step. Reload has
explicit unchanged/stale/refused/activated outcomes rather than unconditional
generation churn. Remaining design work is to finish one prepare/admit/publish
contract across reloadable registries and settle the permanent moveset authoring
source. See [I2/I3](queue.md#i2i3--finish-independent-content-authoring-and-safe-reload),
[Q104](awaiting-maintainer-decision.md#q104--is-the-rust-move-table-or-the-content-file-the-source-of-a-moveset)
and [Q110](awaiting-maintainer-decision.md#q110--may-a-provider-keyed-fragment-registry-gain-a-named-hot-reload-replacement-operation).

### Composition and public profiles

A9 remains the owner for truthful minimum engine profiles. The target is a named
capability contract, not a crate-count budget. Current product/architecture choices
that shape the profile are Q97, Q100, Q106 and Q108 in the decision ledger.

⛔ **AND A PROFILE WITNESS MUST STEP, WHICH UNTIL 2026-09-16 NONE OF THEM DID.**
The three composition probes passed in under half a second having run ZERO fixed
steps, because `MinimalPlugins` leaves `TimeUpdateStrategy::Automatic` and a fast
run never crosses 1/60 s. They certified that the engine BUILDS. A profile
contract tested by an arm whose green is compatible with the engine being broken
is not a contract. ⇒ Pin the step, then ASSERT the step happened — the pin alone
fails silently.

## Current execution

The queue is intentionally compact. Its current groups are:

- **P0:** A10 publication (CLOSED), peer-stable identity (**no road count here —
  the owner row re-derives it, and this copy read "eleven of fourteen" after the
  owner had moved to fourteen of seventeen.** The three open roads are `Q128`,
  `Q122` and the unchecksummed float rows — the last a different kind, with no
  host-local id, never compared between peers, blocked on netcode's N2. All three
  want a maintainer or a P2P session, so none is pickable here), settings/rollback
  policy, throw modifier
  consistency, A2 projectile identity (CLOSED — the construction-identity hole
  only; A2a/A2b/A2c geometry and contact contracts are a different subject and
  still open), A12 move-contact attribution and A4 control/body execution.

⚠ That list is a SUMMARY OF `queue.md`, which means it is a copy corrections do
not reach. Read the rows, not this line, before picking work up: a group closing
here is a two-edit change and only one of the edits is anybody's job.
⛔⛤ **AND THAT WARNING FAILED TO PROTECT THE LINE IT SITS UNDER TWICE, BOTH
TIMES ON 2026-09-16.** First it was corrected above to *nine closed, three open*
while this line still said *"the two open are `Q128` and `Q122`"* — written by
someone reading this exact warning. Then both copies drifted again: the section
above counted a fourteenth road and thirteen live while `queue.md` had fifteen
filed and fourteen live, and this line still said nine closed and four open after
the input payload closed for its ratchet. ⇒ A warning that a copy will rot does
not stop it rotting, and neither does the habit it recommends. The only reading
that survives is the one that RE-DERIVES: `queue.md`'s ID-PEER row prints its own
arithmetic (nine in the table plus two in prose beneath it) precisely so a reader
does not have to trust a sentence anywhere — including that one.
- **P1:** content reload, A9 composition, item occurrence ownership, fighter-brain
  selection, low-tier sprite policy, Smash parity, character authoring and
  scenario identity. ⚠ Test-lane reliability is no longer a standing P1 theme:
  the long-running `app_it` failure was a sim-schedule cycle, not a flake, and
  the lane runs. What remains in TEST-LANES is the fails-in-company class —
  THREE instances, of which the third is closed with a measured cause (a per-App
  page census keyed on a process-global's colliding asset id) that does NOT
  explain the other two — plus one older non-reproducing session-root handoff
  failure whose assertion was never captured. ⇒ Read the rows: `queue.md`'s OPEN
  1 states which candidates are refuted (CPU contention; a settle loop going
  quiet over a growing set) so nobody re-measures them.
- **P2:** product/authoring work that has an executable owner after a maintainer
  rule.
- **P3:** measurements that require a particular machine, device or interactive
  runtime.

If a row closes, remove it from the queue unless another open row needs a short
receipt.

## Evidence discipline

Architecture documents distinguish source facts, inferred source behavior and
claims that require compilation/runtime verification. Do not convert a static
observation into a runtime claim. Re-run the measurement that supports a current
number before using it as a new baseline.

The architecture census uses:

- `SOURCE_CONFIRMED`
- `SOURCE_INFERRED`
- `DOC_CLAIM`
- `NEEDS_COMPILED_VERIFICATION`
- `NEEDS_RUNTIME_VERIFICATION`

Counts are campaign observability, not quality budgets.

## Planning control plane

Use the planning tree according to [`README.md`](README.md):

- `queue.md` — executable work only;
- `awaiting-maintainer-decision.md` — unresolved maintainer/product choices only;
- `maintainer-decisions.md` — durable rulings;
- focused owner documents — current authority, topology, executable work and
  acceptance;
- `tracks.md` — standing reservoir, not active execution;
- Git history / `dev/` — investigation chronology and completed campaign history.

Do not recreate archive files inside `docs/`. When a current-state document is
superseded, delete or rewrite it and let Git preserve the old text.
