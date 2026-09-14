# Documentation debloat and consolidation report — 2026-09-14

Source snapshot: `2dbd81abc50f42a601d8e6177478ef0985002365`.

## Summary

This pass cleaned the current planning control plane and two high-value focused
owner documents. It did not redesign engine behavior and did not modify Rust
source.

The governing rule was semantic preservation: keep current invariants, ownership,
lifecycle, failure semantics and executable work; remove review chronology,
closed-work diaries, repeated measurements, stale alternatives and duplicate
architecture explanations.

The largest change is structural rather than stylistic:

- `queue.md` is again an executable queue rather than a case-file archive;
- `awaiting-maintainer-decision.md` contains unresolved maintainer choices rather
  than answered/withdrawn engineering history;
- `status.md` is a short orientation page;
- `tracks.md` is a standing reservoir of enduring claims/triggers rather than a
  second history log;
- the capability-gating and simulation-authority owner docs now state their
  current models directly;
- the consolidation census records the planning-control-plane debt as closed.

## Quantitative result

Measurements are observations, not quality targets.

| Metric | Before | After | Change |
| --- | ---: | ---: | ---: |
| Markdown files in the source tree, excluding this new report | 650 | 650 | 0 |
| Markdown lines, excluding this new report | 142,126 | 129,545 | -12,581 |
| Markdown nonblank lines, excluding this new report | 112,094 | 101,135 | -10,959 |
| Planning Markdown files | 127 | 127 | 0 |
| Planning lines | 48,585 | 36,004 | -12,581 |
| Planning nonblank lines | 39,873 | 28,914 | -10,959 |
| Rust files | 1,908 | 1,908 | 0 |
| Rust full-line comments | 220,469 | 220,469 | 0 |
| Rustdoc lines | 143,386 | 143,386 | 0 |
| Rust TODO/FIXME hits | 91 | 91 | 0 |

High-concentration files changed:

| File | Before | After |
| --- | ---: | ---: |
| `docs/planning/queue.md` | 7,332 | 355 |
| `docs/planning/awaiting-maintainer-decision.md` | 3,160 | 395 |
| `docs/planning/status.md` | 474 | 122 |
| `docs/planning/tracks.md` | 606 | 281 |
| `docs/planning/engine/capability-progression-and-world-gating.md` | 1,136 | 195 |
| `docs/planning/engine/simulation-authority-and-determinism.md` | 1,504 | 293 |

The line reduction is an outcome of removing obsolete narrative. It was not used
as a pass/fail criterion.

## Files modified

Current-planning control plane:

- `docs/planning/queue.md`
- `docs/planning/status.md`
- `docs/planning/tracks.md`
- `docs/planning/awaiting-maintainer-decision.md`

Focused owner documents:

- `docs/planning/engine/capability-progression-and-world-gating.md`
- `docs/planning/engine/simulation-authority-and-determinism.md`

Consolidation census synchronization:

- `docs/planning/consolidation/README.md`
- `docs/planning/consolidation/architecture-census.md`
- `docs/planning/consolidation/campaign-metrics.md`
- `docs/planning/consolidation/consolidation-ledger.json`
- `docs/planning/consolidation/consolidation-plan.md`

This report is the only new file.

## What was removed

### Closed-work chronology from live planning

The queue no longer carries long blocks describing how closed findings were
measured, corrected, remeasured and eventually superseded. Open rows now state:
owner, current state, next implementation, blockers and acceptance.

The status page no longer duplicates the queue's packet history. It provides the
current architecture posture and links to executable owners.

### Answered, withdrawn and engineering-only decision entries

The unresolved-decision ledger no longer carries Q98, Q99, Q111, Q112, Q113,
Q114, Q118, Q120, Q123 or Q124 as open questions. Their durable results belong in
maintainer decisions, owner docs, the current queue or Git history as appropriate.

### Repeated measurement transcripts

`tracks.md` now keeps a durable claim and promotion trigger. Repeated greps,
poison-test stories, dated corrections and volatile counts were removed from the
reservoir. Measurements belong in the focused owner or a repeatable script.

### Duplicate architecture explanation

The capability-gating document now owns one direct description of route gates,
body capability authority, participant/world facts and open product choices. The
simulation-authority document now owns one direct description of rewind
participation, deterministic identity, rollback registration, session authority
and extension-state interaction.

### Obsolete planning state

The consolidation census previously said the live queue still mixed executable
work with historical case files. That statement is now a closure receipt rather
than a stale current defect.

## Fixups made while consolidating

### Q127: damage-policy lifetime now has an explicit maintainer decision

The old queue said `PlayerDamagePolicy` still required a maintainer ruling about
match-wide versus participant-specific policy, but no live question owned the
choice. Q127 now does. The queue and status page link to that decision instead of
leaving an implicit blocker.

### Current planning roles now agree

`queue.md`, `status.md`, `tracks.md` and the decision ledger now implement the
roles already stated by `docs/planning/README.md`. This avoids needing another
meta-document to explain which copy of planning state is authoritative.

### Stable consolidation IDs retained

The census entry `TRANS-PLANNING-HISTORY` remains in the machine ledger as a
closed stable-ID receipt rather than disappearing and later being rediscovered as
an apparently new problem. `DOC-QUEUE` now describes the restored current role.

## What was deliberately preserved

The cleanup retained explanations needed to avoid architectural regressions,
including:

- candidate-world and strong last-good-world requirements;
- local lifetime/correlation identity versus peer-stable mechanical identity;
- rollback registration and resimulation ownership;
- mechanical edit proposal/admission/publication boundaries;
- immutable content generations and stale-work refusal;
- session ownership and narrower-than-App lifetimes;
- construction/publication failure semantics;
- capability installation versus optional composition;
- current product decisions that genuinely block engineering;
- explicit production-path acceptance criteria.

The two focused owner rewrites also preserve limitations that static inspection
cannot prove as runtime facts.

## Deferred areas

### A10 hot files

`docs/planning/engine/actor-monolith-work-frontier.md` remains large and contains
historical narration, but it is the active A10 work frontier. Rewriting it during
this pass would create avoidable conflict and risk deleting details still being
used by the implementation campaign. Debloat it after A10 stabilizes.

### Smash parity inventory

`docs/planning/demos/smash-parity-inventory.md` is still a major chronology
concentration. It is also an active feature/status inventory with product-tuning
facts embedded inside its historical corrections. It deserves a separate
semantic pass that rewrites each row to current status rather than bulk-deleting
dated text.

### Rollback/checkpoint and custody owner docs

`runtime-frame-history.md`, `checkpoint-restoration-protocol.md`,
`item-custody-and-accounting.md`, `authored-technique-admission.md` and related
architecture-sensitive plans still contain dated evidence. They were left intact
because those documents encode rollback, persistence and admission invariants.
The next pass should consolidate one owner at a time and verify every retained
current claim against source.

### Moveset inspector

`moveset-inspector.md` remains the largest planning file. Size alone does not show
that its detailed geometry/art observatory design is redundant. The reservoir was
changed to point at its current remaining definition rather than copying the
investigation history.

### Rust comments and rustdoc

No Rust source comments were changed in this docs-focused pass. Architecture-hot
source is changing in parallel, and comment cleanup there should be reviewed
independently from behavior changes. The baseline above is retained for a later
source-comment pass.

### Historical/incubation material

`docs/brainstorms`, `docs/learning`, recipes, engineering journals and ADRs were
not shortened merely because they are large. They are not the live planning
control plane, and historical/design exploration can be legitimate in those
homes.

## Suspicious architecture found incidentally

This pass did not change production architecture. One control-plane omission was
found and fixed as Q127: deterministic damage policy had an engineering blocker
without a maintainer-decision owner.

The capability owner also still records a transitional direct body-capability
grant in the falling-sand path. That is an architecture observation for its owner,
not something this documentation cleanup should refactor.

## Validation

- `git diff --check` is required before packaging.
- `consolidation-ledger.json` parses as JSON.
- The planning-citation checker reports all citations in the changed planning
  files resolved among repositories it can read. Two submodules in this archive,
  `dev/ambition_dev_measurements` and `game/ambition_map_assets`, are not
  initialized, so the checker cannot give a repository-complete strict pass.
- The repository-wide documentation-link checker still reports only the two root
  links to `.agent/README.md`. This source archive does not contain the generated
  `.agent/README.md`; the cleanup does not rewrite those intended inventory links
  to hide an archive-generation omission.
- No Rust compiler or runtime claim is made by this campaign.

## Follow-up order

For another debloat pass, use this order:

1. wait for A10 to stabilize, then rewrite the actor-monolith frontier to current
   packets and closure criteria;
2. give `smash-parity-inventory.md` a row-by-row current-state pass;
3. consolidate rollback/checkpoint/custody owner docs one semantic owner at a
   time;
4. only then run a source-comment/rustdoc pass, keeping invariant comments at the
   abstraction that owns them.

Do not reopen the queue/status history to preserve the deleted narrative. Git is
the receipt.
