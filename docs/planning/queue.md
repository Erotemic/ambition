# The queue — live execution order

This file is the **current executable engineering queue**. It is not a work log,
review transcript, campaign archive or place to preserve completed investigations.
Git history owns those records, including intentionally retired epochs in
[the cold history store](repository-history.md).

A row stays here only when an engineer can act on it without first reconstructing
weeks of context. Durable design belongs in the linked owner document. Product
questions belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

**Architecture review source:** `300004d601af1e633cfaee969f079cf9bb368ca8`
(2026-09-08 committed archive). Revalidate before changing a newer head. The
review made no Rust execution claim; see the coverage receipt.

## P0 - characterize and repair current correctness gaps

### A1a - DONE 2026-09-08

`restore_checkpoint_on_session_start` latched `routed_for` before asking the
lifecycle slot and threw the `Admission` away, so a refused crossing spent the
session's one resume and stranded the player in the room the session opened in.
Fixed by latching only on `Admission::admitted()`; guarded by
`a_refused_slot_leaves_the_checkpoint_resume_retryable` (poison verified) and the
missing-subject arm beside it. A1b then moved the state, systems, installation
and tests out of `shrine`/item-pickup into `session::checkpoint`. F9's executed witness landed
with it: on a refused reset the entitlement ledger rolls back **and the object
acquired after the checkpoint is destroyed outright**, because custody
restoration and the room reconstruction that would re-author it fall on opposite
sides of an admission neither consults. Measurements and the A1c obligation are
in the [protocol](engine/checkpoint-restoration-protocol.md#f9-measured-2026-09-08-executed-full-checkpoint-horizon-composition).

**Standing prohibition:** no consequence of a lifecycle request may be written
before the slot has said yes.

### A2 - unify projectile contact geometry and obstruction semantics

**Owner:** [projectile contact protocol](engine/projectile-contact-protocol.md),
packet A2 and findings F2/F3.

Establish shared geometry, actual travel legs and finite-shape obstruction order
before replacing family dispatch. Preserve synchronous interception and later
hit reception. Carry collider contributor identity: a destructible's own wall
and its hurt shape can be one compound contact, not competing unrelated targets.

**Acceptance:** authored-empty geometry, thin wall/target, equal-time ties,
compound solid object, reflection/absorption, returning shots and rollback have
explicit production-road outcomes. The initial sampled-target sweep is not a
claim of full moving-target CCD. No second family query chooses the victim.

### A11/A12 - make authored technique admission truthful and bounded

**Owner:** [authored technique admission](engine/authored-technique-admission.md).

Start with A12a's raw validation and A11a's installed-profile declaration. Then
validate every expanded effect site, install the private prepared representation
and prove explicit activation. Version-1 flows are acyclic, 1-256 nodes and use
checked indices/finite waits. Keep current move lifetime and delivery phases.

**Acceptance:** invalid/uninstalled calls cannot publish definitions; rejection
leaves active generation unchanged; existing 3-/4-node flows retain their traces.
Finish does not remove recovery, Wait does not extend the move, and late contact
feedback cannot mutate another move occurrence. No generic execution registry.

## P1 - ownership and independently testable composition

### A1c - finish the selected restore through one commit boundary

**Owner:** [checkpoint restoration protocol](engine/checkpoint-restoration-protocol.md).
A1b (ownership move) and A1c subcommits 1-2 landed 2026-09-08: no domain reads
the raw `ResetToCheckpoint` any more, a refused request changes no domain state,
a refused request is remembered rather than lost, and a no-item checkpoint
composition works.

Subcommit 3's preparation half landed 2026-09-08: the accepted operation now
outlives its frame, carries the occurrence/minted inputs pinned at admission, and
room loading derives both the prefetch cache key and the fresh plan from it
instead of from a live ledger the restore had swapped in order to be read.

Subcommit 3b landed 2026-09-08: destructive domain application left ordinary
speculative simulation for `CheckpointDomainApply`, a schedule only a commit
executor runs, with its inputs installed for that schedule's duration and removed
on every path. Both executors are witnessed and poison-verified. The transitional
`AdmittedCheckpointRestore` token was deleted rather than given a longer
lifetime: authorization is structural now.

The accepted operation now has an identity — `CheckpointOperationKey`, the
session's ownership stamp plus an admission-only sequence — and every stage after
the transaction opens matches on it rather than on intent equality.

Verification, terminal outcomes and startup unification landed 2026-09-08: the
commit checks the applied world against the snapshots the operation was accepted
with, publishes exactly one outcome per key, blocks gameplay on failure, and
startup routing is now the same operation mechanism —
`CheckpointResumeProgress` is deleted rather than renamed.

Verification now covers the ledger, the bag, the room the operation claimed to
reconstruct, the subject it restores around, and — per banked custody row — that
the occurrence is carried, carried ONCE, and carried by the custodian the
checkpoint names. Each has a case that fails for it alone. The operation
counter's overflow path refuses the lifecycle slot rather than the identity
(poison-verified), the key has one canonical projection, and the terminal outcome
is a closed `RestoreFailure` rather than a free-form string.

The acceptance matrix was audited row by row on 2026-09-08 and the result is in
the protocol. Fifteen of seventeen rows have a witness that fails for that row's
property.

**Remaining before A1 closes:**
- ONE matrix row is not honestly closed: the end-to-end prefetched-plan arm,
  blocked by a COMPOSITION MISPLACEMENT rather than by content or test-writing.
  `prefetch_neighbor_room_preparation_system` braids a construction-plan cache
  together with the neighbour's ASSET residency in one function, and is installed
  by a PRESENTATION plugin — so the headless harness every checkpoint fixture
  uses prefetches nothing at all. See the protocol's note: it is a SPLIT, not a
  move (the asset half belongs where it is), it is its own packet, and A1 does
  not own it. (The
  preparation-FAILURE row closed differently: since A1c/3b nothing destructive
  runs before the commit, so "live data unchanged" is a consequence of where the
  reducers are registered rather than a behaviour to test.);
- ⛔ verification's remaining omissions are DELIBERATE, not pending: body
  placement has no comparand but the arrival, which transit legitimately
  reconciles off, and clocks/portals have no accepted snapshot at all. Checking
  either needs a new decision (a transit postcondition; a snapshot of what a
  checkpoint means for a clock), not another assertion. Population completeness
  landed with its own failing case;
- the terminal outcome has no presentation consumer. When one is wanted, publish
  a message at completion rather than polling the session's single-latest-outcome
  resource.

**Acceptance:** preparation/prefetch read the pinned snapshot rather than a
modified live ledger; a checkpoint change invalidates a prefetched plan that
would produce a different population even at the same target room; final
verification and rollback rebase include restored custody/occurrences. A failed
destructive native apply remains fail-closed, not an invented undo guarantee.
Retain the corrected actor-spawn boundary.

### A3 - relocate actor-specific world placement lowering

**Owner:** prepared construction integration; packet A3.

Move `ActorPlacementContext` and its actor/catalog-facing lowering adapter toward
construction. Keep generic world provider vocabulary and spatial facts with
world. This can proceed independently after its packet preflight; there is no
required numeric SCC predecessor.

**Acceptance:** world no longer imports actor preparation solely to lower a
placement; provider validation, body construction and failure behavior remain
covered. No executable type-erased recipe registry replaces the direct adapter.

### A9 - establish truthful minimal engine profiles

**Owner:** public SDK and composition; packet A9.

Record the Cargo feature closure of real external fixtures. The source-only
no-default-feature lower bound still contains 51 other workspace packages,
including render through host. Separate compiler reachability, runtime
installation and public-import ergonomics; repair one dependency path at a time.

**Acceptance:** a supported profile constructs and steps a real subject, its
promised absent capability is absent from both installation and resolved closure,
and the full Ambition composition continues to work.

### C2 - capability-owned installation, only where ownership is established

**Owner:** [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

Fresh source instruments report 0 capability/ruleset private orderings,
73 composition private orderings, 174 foreign installations, and 3 mechanically
reducible versus 38 irreducible installation blocks. These are locator metrics.

Move a reducible block only after establishing one implementation owner. Keep
cross-capability policy in explicit composition. Resolve Q73 before adopting a
plugin form that conflicts with the recorded combat convention; an owner helper
can be sufficient. Zero foreign installations is not the target.

**Acceptance:** public set ancestry and deferred visibility are covered, optional
capabilities remain optional, and no broad runtime policy object replaces imports.

The remaining A4-A8/A10 packets have evidence-based holds in the frontier. They
are not executable queue commitments. SCC counts do not release those holds.

## P2 — current engine/game work

### D-TETHER-LINE — give the ledge tether a readable generic reach line

**Owner:** [`engine/expressive-move-capabilities.md`](engine/expressive-move-capabilities.md).

The reel is mechanically visible through movement but has no attachment line.
Do not teach `sim_view` about the Smash-specific `TetherReel` component. Publish a
generic body-to-world reach/attachment fact from gameplay and let the existing
presentation line road consume it.

**Acceptance:** diagonal ledge tether draws from body to actual anchor, Performer
flyline/grab reach remain correct, and no engine/view crate imports Smash ruleset
state.

### D-POTATO-ASPECT — resolve tier-dependent character trim/aspect drift

**Owner:** [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).
**Maintainer choice:** Q69 in the decision ledger covers the interim `0_25x`
fallback policy.

**Do:** keep generated-tier measurement explicit; do not turn missing ignored
manifests into a pass. Fix generation/trim semantics so a selected tier preserves
the promised frame geometry.

**Acceptance:** same authored frame at full/half/quarter/potato has bounded
anchor/aspect drift; potato does not receive fewer drawable pixels than its
selected quality contract promises.

### D129 — finish authored-geometry sprite clipping repair

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md)
and current renderer geometry contracts.

Work from the current measured clipped-sheet population, not historical counts.
Fix per character/sheet where authoring is genuinely wrong; do not introduce a
global scale heuristic to erase asset mistakes.

**Acceptance:** render-time clipping warning population decreases for intentional
repairs and unchanged composited/tiling cases stay classified rather than hidden.

### D-BRAIN-MENU — make the fighter brain able to order from its authored move menu

**Owner:** [`engine/fighter-brain.md`](engine/fighter-brain.md).

The generic fighter brain can have legal authored attacks that its scoring shape
never selects. Implement the owner doc's current scoring/menu packet; do not add
per-character special-case button scripts.

**Acceptance:** representative CPU can select movement-compatible attacks,
smashes/charged options become live customers where the authored menu permits
them, and easiest difficulty remains intentionally poor rather than suicidal.

### D72 — continue Smash parity from the inventory, not a campaign diary

**Owner:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md).

Choose the highest-priority remaining parity row whose primitive is not blocked by
a maintainer decision. Implement through reusable engine capability when the move
class is reusable; demo-only policy stays in Smash.

**Acceptance:** update the inventory row and add production-path acceptance. Do
not append another chronology to a retired expressive-moves campaign.

### D166 — make character authoring boundaries load-bearing

**Owner:** [`engine/character-authoring-package.md`](engine/character-authoring-package.md).

Continue only from the owner doc's measured census. Migrate a field when there is
a real duplicate/competing authoring authority, not because a struct looks large.

**Acceptance:** one authoritative authored value, all runtime projections derive
from it, and the old duplicate path disappears.

### D-SCENARIO-IDENTITY — finish scenario identity transport/cache ownership

**Owner:** performance/scenario tooling.

The report-side identity distinction is complete; remaining work is transport and
cache identity. Unsupported geometry must continue to refuse rather than staging
Flat data under a different scenario name.

**Acceptance:** two scenario geometries with the same benchmark knobs do not
share a cache/result identity; unsupported geometry exits as unsupported.

### D-PORTAL-INTERACT-SEAT — finish interaction input layering after per-body arbitration

**Owner:** control/input composition.

Per-body arbitration is done. Remove any remaining gameplay-input dependency on a
portal/ruleset-specific presentation decision. Interactions for multiple driven
bodies must be served once each despite deferred despawn.

**Acceptance:** two driven bodies can independently interact in the same tick,
portal presence does not change which semantic interaction intent exists, and no
query-order `.next()` arbitration returns.

### D-ID-CONVENTION-DRIFT — keep shared semantic key builders single-owned

**Owner:** registry/identity owners.

Continue only when a producer and consumer still construct the same semantic ID
with separate format strings. Move spelling into the semantic owner and update all
customers in one change.

**Acceptance:** grep finds one constructor for the migrated key family and both
producer/consumer tests use it.

### D-BUILD-GRAPH-BLINDNESS — keep optional/dependency measurements non-vacuous

**Owner:** build/architecture tooling.

Do not interpret declaration count as capability reachability. Measurements must
resolve definitions, feature conditions and actual closure.

**Acceptance:** fixtures distinguish declared-but-unused, feature-gated and
actually linked dependencies.

### D-LANE-UNRUNNABLE / D-APPIT-FLAKE — preserve executable test lanes

**Owner:** test runner / app integration lane.

When a lane cannot run because the environment lacks a precondition, report
**incomplete**, not pass. For flakes, isolate the production ordering/state source
instead of increasing retries.

**Acceptance:** missing Cargo/target/GPU prerequisites are explicit receipt states;
known deterministic fixtures do not depend on wall-clock or entity order.

### POST-CARVE-DOC-SWEEP — update moved-source references in the same carve

**Owner:** the carve author.

Run citation/link/source-reference guards on the **diff** after a move. Re-tense
historical prose where useful; delete live directions to old paths. Do not retain a
huge global post-carve diary.

## P3 — human-gated measurements and local-machine work

These rows cannot be completed from an ordinary headless source review. Keep the
measurement here; keep analysis/results in the owning tool or owner document.

- **D-RASTER-3:** run the remaining weak-GPU framebuffer-scale versus source-tier
  experiment on the intended GPU. Owner: `engine/performance-and-iteration.md`.
- **Switch Pro outer range:** run the controller diagnostic on both machines and
  record the measured radial maxima/dead-zone behavior before changing stick
  thresholds.
- **Web reveal branch:** validate the existing reveal-barrier branch on the real
  browser/GPU target before merge; do not infer from native first-draw behavior.
- **Kaleidoscope Bevy-0.19 flash:** reproduce interactively before filing a fix;
  stale no-repro descriptions are not a queue substitute.
- **LDtk preview tilesets:** measure whether editor-preview assets still decode the
  full player sheet on current boot before changing residency policy.
- **Capture stays alive after window close:** reproduce with current capture
  tooling and identify the live owner keeping the process alive before patching.
- **External consumer/platform checks:** use the SDK/external-consumer owner docs;
  do not claim portability from in-workspace fixtures alone.

Product/content choices such as dense-room composition, evergreen settings,
camera legibility limits and asset policy live in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md), not here.

## Replenishment rule

Add a queue row only when all of these are known:

1. the current production failure or missing capability;
2. the semantic owner;
3. the next concrete edit or measurement;
4. an acceptance test/receipt that can falsify the work.

If one is unknown, put the question in the relevant owner document or maintainer
decision ledger instead. When a row is complete, delete it from this file. Git
history is the completion log.
