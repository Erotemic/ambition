# Planning status

This page is a short orientation snapshot. Live execution is in
[`queue.md`](queue.md). Product rulings are in
[`maintainer-decisions.md`](maintainer-decisions.md); unresolved choices are in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). Durable
architecture belongs in the focused owner documents.

Do not copy detailed packet history into this file. Git history is the receipt for
completed work. Do not copy counts from a live queue row into this page: a copy
does not receive the corrections that the owner row gets.

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

A10 is closed. Room and session state are candidate-owned: validation precedes one
publication switch at both scopes, and outgoing state retires only afterward. A
verified publication also freezes the effects it owes the world outside its own
population, so a room published inside a pending candidate session announces
nothing to the live one until that session is admitted. There is no active A10
lane. The row is
[A10 in the queue](queue.md#a10--candidate-world--last-good-world-publication---done-demolition-closed-2026-09-16).

Consolidation gates that followed from A10:

- C03 and C06 are startable. C03 is a migration of ownership, not of fields:
  under the `Q132` ruling there is exactly one canonical live `SessionRoot`, and
  session-dependent mutable state must carry explicit scope rather than anonymous
  App-global identity.
- C05 is decided: do not start. Its authority collapse has already happened; the
  remainder is one value's storage kind with no defect behind it.

The rulings and their evidence live once, in
[`consolidation/consolidation-plan.md`](consolidation/consolidation-plan.md) and
[`maintainer-decisions.md`](maintainer-decisions.md).

### Deterministic identity

Peer-stable identity (ID-PEER) is a separate campaign. Its open roads are blocked
outside the campaign, on maintainer rulings or on a P2P session this workspace
does not construct:

| open road | blocked on |
| --- | --- |
| the snapshot schema fingerprint hashing English prose | not blocked — [ruled](maintainer-decisions.md) (`Q122`): mechanical identity fingerprints mechanical facts, not explanatory prose. The naive fix is refuted in the row |
| the unchecksummed float rows | netcode **N2** for the whole class. They carry no host-local id and are never compared. The state half is measured by two-host differing-history arms; the count is in the owner row |
| the canonical timeline itself (absolute `SimTick`) | [Q128](awaiting-maintainer-decision.md#q128--should-the-simulation-tick-be-rebased-when-peers-agree-to-start-or-stay-an-absolute-per-app-count) — a projection excluding the tick would exclude the timeline |

The per-road table, the arm that holds each closed road, and every measurement are
in [ID-PEER](queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity).
This section deliberately carries no road count.

Local lifetime/correlation identity and peer-stable mechanical identity remain a
separate seam. `SessionScopeId`, shell activation ids, content epochs and
monotonic counters stay valid for cleanup and stale-message rejection; none of
them may determine authoritative RNG, deterministic construction provenance,
rollback identity, contact/projectile identity, or a peer checksum.

No session in this repository observes a real peer: `SyncTestSession` is the only
one constructed. A test can compare two hosts, though:
`the_peer_visible_surface_does_not_record_which_route_the_host_visited_first`
builds two hosts with different route histories and compares only the
registrations that feed the peer checksum. Defects in this seam have been found by
reading what the pinned dependency actually hashes and by deleting fallbacks, not
by a clean census. That is why this seam gets more review than any current test
can justify.

### Rollback-safe mechanical editing

Live mechanical editors use an explicit proposal/admission/publication boundary
rather than writing simulation authority directly. The census records six current
editor domains. Remaining work should extend that protocol when a new mechanical
editable domain appears; it should not create a second editor-specific rollback
road.

Mutable user preferences are distinct from admitted mechanical policy. Direct
`UserSettings` reads have been removed from the simulation schedule. Difficulty is
[ruled](maintainer-decisions.md) (`Q127`): no generic engine difficulty
architecture; difficulty is game policy as presets, handicaps and CPU brain levels
are separate concepts, and the topic is deprioritised until the default game plays
exceptionally well.

### Persistence and the peer contract

The save mirrors and the dialogue visit count now run in the simulation schedule,
so a rewind replays them; no hashed-save writer remains outside the rewind window.
Disk I/O stays outside the simulation. The measurement and the repair live in
[DURABLE-HORIZON-CHECKSUM](queue.md#durable-horizon-checksum--the-save-mirrors-write-hashed-state-from-update)
and
[ROLLBACK-BAG-DESYNC](queue.md#rollback-bag-desync--ambitiongamesave-disagrees-with-its-own-rollback-replay---repaired-2026-09-16-acceptance-met-the-authorityrepresentation-split-is-deferred-and-q129-is-open).

Almost every system that writes `AmbitionGameSave` is in a rewinding schedule, so
the save is simulation state in practice. Removing it from the checksum is
therefore the large option, and it was refused: it would discard comparison
coverage for real simulation state. The census is owned by `queue.md`.

Open: [Q129](awaiting-maintainer-decision.md#q129--must-the-save-file-be-part-of-what-two-peers-agree-on)
asks whether a save file should be part of what two peers agree on.

Ruled (`Q138`): an invalidated harness must refuse or fail rather than silently
produce frozen observations. `scripts/a_rollback_arm_must_refuse_a_frozen_world.py`
runs in `--maintenance`: each sync-test arm reads a health API or states what a
frozen world breaks in it.

### Content generations and fast iteration

The host can consume edited move content without a Cargo/link step. Reload has
explicit unchanged/stale/refused/activated outcomes rather than unconditional
generation churn. Remaining design work is to finish one prepare/admit/publish
contract across reloadable registries and settle the permanent moveset authoring
source. See [I2/I3](queue.md#i2i3--finish-independent-content-authoring-and-safe-reload).
Rulings ([`maintainer-decisions.md`](maintainer-decisions.md)): content-authored
movesets are the long-term authority and duplicate Rust tables are migration
scaffolding (`Q104`); mechanical registry changes use explicit
lifecycle/replacement semantics rather than a universal silent overwrite (`Q110`).

### Composition and public profiles

A9 remains the owner for truthful minimum engine profiles. The target is a named
capability contract, not a crate-count budget. Current product/architecture choices
that shape the profile are Q97, Q100, Q106 and Q108 in the decision ledger.

A profile witness must step. `MinimalPlugins` leaves
`TimeUpdateStrategy::Automatic`, and a fast run never crosses 1/60 s, so an
unpinned probe runs zero fixed steps and only proves that the engine builds. Pin
the step, then assert that the step happened.

## Current execution

For what is blocked, read the section *What actually blocks architecture work
today* in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).
It is derived from `queue.md`'s `**Blocked by:**` fields, its prose gates and
every `DO NOT START BEFORE` in the consolidation plan.
`scripts/check_blocking_set_names_every_gate.py` keeps the two in agreement.
This page does not restate that set or its size.

The queue is intentionally compact. Its groups are listed below. This list is a
summary; read the rows in `queue.md` before you pick up work.

- **P0:** peer-stable identity (ID-PEER; its open roads want a maintainer or a
  P2P session), settings/rollback policy, throw modifier consistency, A2a/A2b/A2c
  projectile geometry and contact contracts, A12 move-contact attribution and A4
  control/body execution.
- **P1:** content reload, A9 composition, item occurrence ownership, fighter-brain
  selection, low-tier sprite policy, Smash parity, character authoring and
  scenario identity. TEST-LANES holds the fails-in-company class; its instances,
  cause and remainder are on
  [its triage page](triage/a-composition-acceptance-that-only-fails-in-company.md).
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
