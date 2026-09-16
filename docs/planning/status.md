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
~206 `Single<.., With<SessionRoot>>` sites run or skip. ⇒ **C03 can start; it
should not MOVE STORAGE before Q132 is answered**, because that ruling decides
whether a two-root frame may exist at all.

The row is [A10 in the queue](queue.md#a10--candidate-world--last-good-world-publication).

### Deterministic identity

Local lifetime/correlation identity and peer-stable mechanical identity remain a
separate active seam. `SessionScopeId` is useful as an App-local session owner,
but local activation counts must not determine peer-stable provenance or
canonical checksums.

**NINE CLOSED, THREE OPEN (2026-09-16).** The per-road table and the arm that holds
each one are in
[ID-PEER](queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity).
⭐ This page said "nine of the ten" and predicted that a tenth road found
tomorrow would make it "nine closed, a tenth found" instead of making it false.
An eleventh was found on 2026-09-16 and the sentence held: the count moved, the
claim did not. A **twelfth** was found later the same day and the sentence held
again — but this one is a DIFFERENT KIND of road, which is worth saying because
"twelve roads" would otherwise imply twelve of the same thing.

⭐ **AND A THIRTEENTH WAS FILED AND WITHDRAWN THE SAME DAY, WHICH IS THE
CAMPAIGN WORKING.** Walking `possession_trigger_system`'s inputs found an
App-local, menu-mutable USER PREFERENCE interpreting GGRS-replayed stick input
inside the simulation — measured and real, and already owned by
[SETTINGS-ROLLBACK](queue.md#settings-rollback--finish-the-settingsmechanics-admission-boundary)
with a better repair recorded than the one about to be proposed. A new row would
have been a second owner for one fact. ⇒ What survived is the peer half (that
owner had only the local half) and a correction to that row, which said the
frame-mode side was *"projected"* in its done clause while the waiver eleven lines
from the code says a resimulation still reads today's policy. **"Projected" is not
"admitted"**, and both policies — `PlayerDamagePolicy` and
`SeatControlFrameModes` — are written from `Update`, unregistered for rollback, and
read by sim systems.

⛔ **THE TWELFTH ROAD IS NOT A LINEAGE DEFECT AND NO PROJECTION FIXES IT.** The 25
rows ranked by **S7** in
[simulation-authority-and-determinism.md](engine/simulation-authority-and-determinism.md)
are outside the session checksum, read by an unfiltered per-tick query, and
float-bearing; 12 of them are mutably written in production. They carry no
host-local id — they are simply never compared between peers. ⇒ And they cannot be
measured here: `Session::SyncTest` is the only session this workspace constructs,
so the two rows measured clean (`item.ground_item`, `actor.animation_facts`) are
cleared of a local RESTORE defect and say nothing about two peers. The blocker is
netcode's **N2**, the absent P2P session — the same blocker `Q128` has under
another name.

⛔ **NEITHER OF THE OTHER TWO OPEN ROADS IS A SESSION COUNT, AND BOTH ARE BLOCKED
ON A RULING.**
[Q128](awaiting-maintainer-decision.md#q128--should-the-simulation-tick-be-rebased-when-peers-agree-to-start-or-stay-an-absolute-per-app-count)
is the absolute `SimTick`.
[Q122](awaiting-maintainer-decision.md#q122--which-registry-fields-are-mechanical-and-which-are-presentation)
is the snapshot schema fingerprint hashing English prose: `compute_schema_fingerprint`
hashes the whole `schema_dump()`, `detail` column included, so two builds of the
same mechanical schema are two identities if somebody reworded a comment.
Measured by poison — one pluralised word moved 83 rows.
`SimTick` is registered `resource-canonical`, so its whole value is compared
between peers, and it is an absolute count of every sim step an App has run
(one writer, unconditional at the head of the schedule, never rebased, menu
frames included). Two Apps running for different lengths of time therefore
disagree from the first compared frame. It cannot be closed the way the other
nine were: a projection excluding the tick would exclude the TIMELINE, which is
what a rollback comparison is about.

⚠ Nothing in the repository can observe any of this: the only sessions in use are
`SyncTestSession`, one machine rewinding itself, and a canary comparing a machine
against its own past cannot catch a two-peer disagreement. Every road in that
table had to be found by reading, and two of the three GPT reviews found a fix
that had replaced one host-local term with another.

⭐ **THE TWO THAT CLOSED LAST NEEDED NO NEW AUTHORITY**, which is the transferable
part: the stale cross-session match stamp needed to be impossible rather than
checksummed harder, and the session root's identity needed no peer-stable session
term at all — its local count was disambiguating nothing. Ask who ORDERS and who
OWNS a thing before designing a type to carry it.

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

⛔ **MEASURED 2026-09-16: THE SAVE FILE IS INSIDE THE PEER CHECKSUM AND IS
DERIVED PER FRAME.** `AmbitionGameSave` is `rollback_resource_clone_checksum`, so
its whole value is compared per TICK, while `persist_inventory_to_save` writes it
from `Update` — per FRAME, and a rewind re-simulates ticks without re-running
`Update`. A bag that changes every tick desyncs a GGRS sync test within six
ticks; of 364 probed rollback entries exactly one diverges, and it is the save.
⚠ A single change does not reproduce it, which is why nothing had hit it.

⚠ This is not only a persistence question: 13 of the 19 systems that write
`AmbitionGameSave` are registered in the SIM schedule, so the save is
simulation-adjacent state in practice whatever it is in principle. That is what
makes "take it out of the checksum" the large option rather than the small one.

The ruling is
[Q129](awaiting-maintainer-decision.md#q129--must-the-save-file-be-part-of-what-two-peers-agree-on);
the measurement and the reproduction are in
[ROLLBACK-BAG-DESYNC](queue.md#rollback-bag-desync--a-per-tick-change-to-an-unhashed-resource-desyncs-the-sync-test).

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

- **P0:** A10 publication (CLOSED), peer-stable identity (nine roads closed; the
  THREE open are `Q128`, `Q122` and the 25 unchecksummed float rows, which are a
  different kind — no host-local id, never compared between peers, blocked on
  netcode's N2), settings/rollback policy, throw modifier
  consistency, A2 projectile identity (CLOSED — the construction-identity hole
  only; A2a/A2b/A2c geometry and contact contracts are a different subject and
  still open), A12 move-contact attribution and A4 control/body execution.

⚠ That list is a SUMMARY OF `queue.md`, which means it is a copy corrections do
not reach. Read the rows, not this line, before picking work up: a group closing
here is a two-edit change and only one of the edits is anybody's job.
⛔⛤ **AND THAT WARNING FAILED TO PROTECT THE LINE IT SITS UNDER, THE SAME DAY IT
WAS WRITTEN.** On 2026-09-16 the ID-PEER owner corrected the "Deterministic
identity" section above to *nine closed, three open* and left this line saying
*"the two open are `Q128` and `Q122`"* — while reading this exact warning. ⇒ A
warning that a copy will rot does not stop it rotting; the only thing that would
have is editing every copy in the same command, which is now the habit for this
page: `grep -n` the claim, not the heading.
- **P1:** content reload, A9 composition, item occurrence ownership, fighter-brain
  selection, low-tier sprite policy, Smash parity, character authoring and
  scenario identity. ⚠ Test-lane reliability is no longer a standing P1 theme:
  the long-running `app_it` failure was a sim-schedule cycle, not a flake, and
  the lane runs. What remains in TEST-LANES is one non-reproducing session-root
  handoff failure.
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
