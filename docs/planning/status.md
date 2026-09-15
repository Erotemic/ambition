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

Post-A10 demolition is the active lane: deleting the mechanisms A10's replacement
made dead. The next architecture campaign is peer-stable identity (ID-PEER),
which owns a separate campaign and a separate agent.

The row is [A10 in the queue](queue.md#a10--candidate-world--last-good-world-publication).

### Deterministic identity

Local lifetime/correlation identity and peer-stable mechanical identity remain a
separate active seam. `SessionScopeId` is useful as an App-local session owner,
but local activation counts must not determine peer-stable provenance or
canonical checksums. The current engineering packet is
[ID-PEER](queue.md#id-peer--remove-host-local-lineage-from-peer-stable-mechanical-identity).

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

## Current execution

The queue is intentionally compact. Its current groups are:

- **P0:** A10 publication, peer-stable identity, settings/rollback policy, throw
  modifier consistency, A2 projectile identity, A12 move-contact attribution and
  A4 control/body execution.
- **P1:** content reload, A9 composition, item occurrence ownership, fighter-brain
  selection, low-tier sprite policy, Smash parity, character authoring, scenario
  identity and test-lane reliability.
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
