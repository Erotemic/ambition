# `docs/planning` — live planning control plane

This directory contains **current work**, not repository history. Git history is
the archive, including intentionally retired epochs in the cold store. Read
[repository history and reconstruction](repository-history.md) before treating a
locally absent commit as lost evidence. The policy applies to future rollovers too.

A planning document earns its place here by answering one of four questions:

1. **What should happen next?** → [`queue.md`](queue.md)
2. **What durable design owns that work?** → a focused owner document under
   `engine/`, `game/`, or `demos/`
3. **What explicit maintainer ruling constrains it?** →
   [`maintainer-decisions.md`](maintainer-decisions.md)
4. **What question genuinely needs a maintainer answer?** →
   [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md)

[`tracks.md`](tracks.md) is the standing reservoir for valuable work that is not
currently selected. [`status.md`](status.md) is a short orientation snapshot.
Neither is an execution diary.

## A gate over a missing document does not fail

`scripts/check_planning_docs_survive.py` asserts the SUBJECT of this tree: the
live control-plane documents exist, are non-trivial, and still carry the headings
that make them what they claim to be. It runs in `--maintenance`.

⭐ **ITS POPULATION IS TEN DOCUMENTS, AND THE FOUR CONSOLIDATION ONES WERE
ADDED 2026-09-16** — `consolidation/README.md`, `consolidation-plan.md`,
`architecture-census.md`, `campaign-metrics.md`. They are where the
architecture campaigns' state actually lives, they get rewritten far more often
than this page does, and every argument for guarding `status.md` applied to them
unchanged while they sat outside the check.

⛔⛤ **IT EXISTS BECAUSE `status.md` WAS EMPTIED TO ZERO BYTES, COMMITTED AND
PUSHED, AND THE LANE REPORTED 7/7.** That is not a hole in the other gates — a
citation checker over a file with no citations has nothing to report, and a hold
checker over a file with no holds has nothing to report. **The absence of
findings and the absence of a subject look identical.** MEASURED by replaying it:
with `status.md` empty the lane is 7/8 and the seven are all still green.

⚠ It is a CATASTROPHE DETECTOR, NOT A RATCHET. Its floors sit far below today's
sizes, because compressing a closed row to a receipt is this contract WORKING —
`queue.md` fell 1063 → 908 lines the day it was written and must not red. And it
cannot see a document that goes wrong while staying big; stale claims, discharged
holds and broken citations have their own checks.

## What else is mechanically enforced over this tree

⛔ **DO NOT LIST THEM HERE. RUN THE LANE AND READ ITS JOB NAMES** —
`./run_tests.sh --maintenance` — because a hand-written list of checks is a
second authority that rots exactly like the claims these checks exist to catch.
The lane is the list.

⭐ What is worth saying in prose is the SHAPE of what they cover, because three
of these classes are ones a careful reader would not think to look for:

- a **gate sentence** naming a campaign `queue.md` marks finished — including the
  prose spelling (*"X should finish before…"*), which is a third spelling of the
  hold convention and was invisible to the first two rules;
- a **number** restated away from its source: the session-owner census is
  compared to `teardown.rs`, and any line in this tree stating that bundle's count
  is swept for a stale copy — a third copy of it survived in a table cell after
  the first two were corrected;
- a **pointer into a heading** — a heading rename breaks every link to it while
  the compiler, the citation checkers and the doc-link job all stay green.

⚠ **AND THE LANE'S OWN BOUNDARY IS PRINTED BY THE LANE.** `--maintenance` does
NOT run `pytest scripts/tests`, which holds the tests of the scripts it runs; its
closing notice derives and names everything it skipped. A green here is a claim
about this lane.

## Where the open work is

[`queue.md`](queue.md) is the one live engineering execution ledger. If work is
not executable yet, it belongs in a focused owner document or the maintainer
decision ledger instead of a second queue.

## Authority order

Separate observations from decisions.

**What the software currently does:** source plus executed behavior take
precedence over a plan. A source inspection is not an executed acceptance test.
Record contradictions instead of editing the observation to fit the design.

**What the software should do:** explicit maintainer rulings constrain design.
The focused owner document records the current engineering decision and its
reasoning; the queue selects work under that decision. An implementation can
violate a ruling without overruling it. An architectural recommendation is not a
new maintainer ruling.

The [architecture reassessment](engine/architecture-reassessment.md) supersedes
the old mandatory P2-P5 SCC sequence and identifies the changed owner contracts.
Use its [responsibility map](engine/architecture-responsibility-map.md),
[bounded work packets](engine/actor-monolith-work-frontier.md) and
[review coverage](engine/architecture-review-coverage.md) together. Its source
findings are tied to the named snapshot and must be rechecked on a newer head.
Unchanged product decisions remain in their existing owner documents.

Three protocol documents now make the highest-risk seams executable:
[checkpoint restoration](engine/checkpoint-restoration-protocol.md),
[projectile contacts](engine/projectile-contact-protocol.md) and
[authored technique admission](engine/authored-technique-admission.md).
They own the detailed state, timing, failure and acceptance rules for A1, A2 and
A11/A12 respectively. Where the earlier broad review offered an unresolved choice,
these focused engineering decisions supersede it. They describe target behavior,
not completed implementation or a new maintainer ruling.

[Fast content iteration and extensions](engine/extension-model.md) owns the
runtime-loaded artifact, procedural SDK and Bevy plugin boundary. Its
[state/execution contract](engine/extension-state-and-execution.md),
[generation/reload protocol](engine/content-generation-and-reload.md),
[domain call contracts](engine/extension-domain-contracts.md),
[implementation packets](engine/fast-iteration-implementation.md),
[acceptance fixtures](engine/fast-iteration-acceptance.md) and
[evidence/experiments](engine/extension-iteration-evidence.md) separate architecture
that can proceed now from backend, storage and timing decisions requiring local
measurement. This supersedes older blanket deferral of runtime extensions until
a public modding customer exists. I3b supplies A10's bounded safe-reconstruction
customer; the long-term game supplies A8's two-instance proof. Neither gates I1/I2.
These contracts refine existing owners rather than create another work queue.

A polished crate name, older campaign, author identity or repeated commentary
provides no additional evidence of responsibility or correctness.

## Direct maintainer observations

There is deliberately **no permanent maintainer-observation dump**.
Direct reports belong with the responsible subsystem, not a personal-name log.

When Jon reports something directly, triage it immediately:

- reproducible engineering defect → `queue.md` plus the owning plan if design is
  needed;
- product/design choice → `awaiting-maintainer-decision.md`;
- explicit ruling → `maintainer-decisions.md`;
- durable product intent → the owning game/system document;
- fixed, superseded, or no longer reproducible → **do not preserve it as live
  planning**. Git history already preserves the report.

Do not create another personal-name scratchpad or general bug dump to replace
it.

## Queue contract

`queue.md` contains executable work only.

- `P0/P1/P2/P3` are priority bands, not permanent identifiers.
- A row names the current failure, owner, next action, and acceptance.
- Deep reasoning belongs in the focused plan. The queue links there.
- When a row is fixed, remove it from the live queue in the same change that
  closes it. The commit and Git history are the receipt.
- If a finding is only a human measurement or maintainer choice, it does not
  occupy an engineering execution slot.
- Re-measure a row before implementing it. A queue row is a claim about a
  changing tree.

The queue should stay short enough that a reviewer can read the whole execution
surface in one sitting.

## Decision contract

`awaiting-maintainer-decision.md` contains only questions that cannot be answered
from current source, existing rulings, or ordinary engineering judgment.

Each entry should contain:

- a stable label;
- the exact question;
- enough context to choose;
- the owning plan;
- what changes after each materially different answer.

When answered, move the ruling into `maintainer-decisions.md` and delete the
question. Do not keep an answered section as a historical receipt.

`maintainer-decisions.md` is likewise compact: decision, date, confidence, and
only the consequence needed to apply it. Investigation history belongs in Git.

## Focused owner documents

A focused plan is healthy when it reads like an owner contract:

```text
scope
current authority
current topology / measurements when needed
executable work
acceptance
forbidden regressions
```

It should not contain the chronology of every failed attempt used to discover
that contract. Once an investigation converges, rewrite the live page around the
result.

Current high-value owner documents include:

- [`engine/extension-model.md`](engine/extension-model.md)
  -- fast content iteration, procedural state and explicit extension tiers;
- [`engine/actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md)
  — bounded ownership migrations;
- [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
  — durable decomposition rules;
- [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md)
  — capability/plugin composition;
- [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md)
  — presentation ownership and body-owned drawables;
- [`engine/item-custody-and-accounting.md`](engine/item-custody-and-accounting.md)
  — occurrence/custody/entitlement semantics;
- [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md)
  — current Smash feature inventory;
- [`demos/moveset-reviews.md`](demos/moveset-reviews.md)
  — maintainer-authored move intent.

## Campaigns

Campaign documents are temporary execution scaffolds. When a campaign closes:

- move durable design to the owner document;
- move still-open work to the queue/inventory;
- collapse the campaign to a short completion receipt if inbound links make the
  path useful, otherwise delete it.

Do not leave a 100 KB completed campaign in the live planning tree because its
chronology may be interesting.

## Verification receipts

Status-bearing documents may carry a dated `Verified against <sha>` receipt when
a pass has actually re-read them against the tree. Doctrine pages do not need a
freshness stamp simply because time passed.

A receipt dates the **check**, not the writing. It is never permission to trust a
stale count instead of rerunning the instrument that owns it.

## Planning hygiene

Before adding more prose, ask whether the fact already has an owner. Prefer:

- one semantic helper over repeated identifier spellings;
- one current measurement plus the command that reproduces it over a timeline
  of old numbers;
- one explicit prohibition over the story of every failed design;
- Git history over an archive section inside the active page.

A planning cleanup may delete large amounts of resolved prose. That is expected,
provided unresolved work, explicit decisions, and durable design survive in the
correct owner.
