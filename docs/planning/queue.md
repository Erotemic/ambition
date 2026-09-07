# The queue — live execution order

This file is the **current executable engineering queue**. It is not a work log,
review transcript, campaign archive or place to preserve completed investigations.
Git history owns those records.

A row stays here only when an engineer can act on it without first reconstructing
weeks of context. Durable design belongs in the linked owner document. Product
questions belong in [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md).

**Reference source:** `625fa79af45e6eff40cbefabd8cdae33c5b5e9db`,
reviewed 2026-09-07. Revalidate source facts before editing a newer head.

## P0 — current correctness and architecture regressions

### D-MARK-ATTRIBUTION-LIFETIME — delayed attacks keep semantic credit after the source body disappears

**Owner:** Smash mark implementation and generic combat attribution.

**Current failure:** `BodyMark` stores the attacker's stable match seat, but
`detonate_body_marks` resolves that seat to a live body and falls back to the
**marked victim** if the attacker's body has already been eliminated/despawned.
In a three-side match, Author can mark Alice, be eliminated, and later have
Alice credited for the blast that hits Bob.

**Do:** preserve semantic attack credit independently from the optional current
source entity. Geometry continues to follow the marked body; `Environment`
continues to define damage eligibility. Do not substitute the victim when the
source body is gone.

**Acceptance:** assembled three-side poison: mark victim, eliminate marker,
let fuse expire beside third fighter, and verify attribution remains the marking
seat. Keep the existing two-attacker refresh rule: the last refresher owns the
mark.

### D-BODY-DRAWABLE-PORTAL-SCHEDULE — finalize body-owned drawable geometry before portal publication

**Owner:** render/presentation scheduling.

**Current failure:** `BodyClockVisual` is a valid `PresentationOf(body)` Sprite,
but `sync_body_clock_visuals` is not ordered/flush-separated before
`publish_portal_compositing_candidates`. A newly spawned clock can render for one
frame without a portal candidate; an existing moving/shrinking clock can be
classified using the previous geometry.

**Do:** publish a semantic presentation phase/set meaning **body-owned drawable
geometry finalized**. Put body clocks, hit-flash frame updates and comparable
drawable producers before it; portal candidate publication consumes it. Do not
accumulate per-overlay `.after(private_function)` edges.

**Acceptance:** partial portal overlap on the clock's first frame and on later
moving/shrinking frames; candidate geometry must match the actually drawn frame.

### D-HIT-FLASH-PORTAL-RELEASE — remove the one-frame far-side to near-side disappearance

**Owner:** hit-flash presentation + portal visibility arbitration.

**Current failure:** the hit-flash owner declines to assert `Visible` while the
previous frame's `PortalSourceHidden` marker still exists. Portal resolution then
removes that marker later in the frame without asserting visibility, leaving the
overlay hidden until the following frame.

**Do:** the normal presentation owner asserts its ordinary desired visibility
each frame before portal resolution; portal reasons may then hide it again.

**Acceptance:** active flash is clipped while far-side and visible immediately on
the first near-side frame. Preserve the new `DeclaredFrame` non-Sprite compositor.

### D-C1-BODY-CLOCK-SET — restore zero private cross-crate capability ordering

**Owner:** body-clock read-model scheduling.

**Current failure:** Smash orders `publish_mark_clocks` after concrete
`sim_view::rebuild_body_clocks_view`. The architecture ratchet is red on this one
foreign private-system edge.

**Do:** publish semantic reset/contribution scheduling vocabulary from the
body-clock/read-model owner and register Smash against that vocabulary.

**Acceptance:** `scripts/tests/test_foreign_system_ordering.py` returns to green
without raising its zero capability/ruleset ceiling.

## P1 — actor-monolith decomposition

### D33 — execute the measured residual-SCC peel

**Owner:** [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
and [`engine/actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md).

Do **P1 through P4 in order**, remeasuring after every packet:

```text
baseline  largest SCC 11
P1        character_runtime -> features       expected 9
P2        projectile -> features              expected 8
P3        shrine -> session                    expected 7
P4        construction -> world                expected 6
```

Then stop source movement and complete
[`engine/actor-monolith-hard-core-edge-ledger.md`](engine/actor-monolith-hard-core-edge-ledger.md).
No six-module-core carve begins with a `TBD` disposition.

The owner docs contain the exact symbols, files, rollback requirements, forbidden
end states and tests. Do not recreate that material in this queue.

### C2 — continue capability-owned installation only where ownership is real

**Owner:** [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

Current measured shape at the reference head:

```text
capability/ruleset foreign private ordering     1   # D-C1-BODY-CLOCK-SET
composition foreign private ordering           73
foreign system installations                  174
mechanically reducible install blocks           2
irreducible composition blocks                 39
```

**Do:** move only the two blocks whose implementation owner is unambiguous.
Composition code is allowed to compose independent capabilities; zero foreign
installs is not the target.

**Acceptance:** owner installer exists, old host/runtime private-system names are
gone, ordering uses public sets, optional-capability/rollback gates still pass.

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
