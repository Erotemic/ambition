# Ambition review guide

Use this guide whenever taking over review, architectural steering, or post-campaign inspection of the Ambition repository.

Campaign handoffs should assume this document has been read and contain only current context: HEAD/baseline, active customer, recent work, known product feedback, suspected regressions to verify, and immediate priorities.

## Role

Act as an independent architecture and correctness reviewer.

The implementation agent's summaries, planning documents, commit messages and test results are evidence. They are not architectural truth.

The job is to determine:

> Is the current work making Ambition easier to extend, author, reason about and use as a game engine, while also making the actual games better?

Do not manufacture work merely to produce findings.

If the current architecture supports the customer cleanly, say so and let implementation continue.

## Start every review from current truth

Before reviewing conclusions from an earlier agent:

```bash
git status
git log --oneline --decorate --graph --all --date-order
```

Establish:

* current HEAD;
* dirty/uncommitted work;
* relevant branches;
* commits since the last reviewed baseline;
* changed runtime/code files;
* relevant submodule heads/gitlinks;
* active campaign/queue state.

Inspect executable code before trusting planning prose.

A recurring failure mode is:

```text
plausible diagnosis
→ later instrumentation disproves it
→ prose survives
→ fresh agent resurrects it
```

Retire findings that no longer exist.

Do not condemn committed code for a correction visibly present in dirty WIP, but state that the correction is not yet reviewable as committed work.

## Review real runtime paths

When behavior crosses systems, schedules or crates, trace the production path.

Do not infer correctness from:

* names;
* helper-unit tests;
* component existence;
* comments;
* green aggregate tests;
* an asset existing on disk.

Examples:

```text
move exists
≠ input can select it

AI selected recovery
≠ recovery succeeded

asset exists
≠ runtime consumes it

component exists
≠ its owner/lifetime is correct

test is green
≠ test exercises the production seam
```

For a cross-system defect, require at least one production-path regression.

## Review priorities

Prioritize:

### Concrete correctness

* behavior wrong now;
* rollback divergence;
* nondeterministic event/query order;
* state restored incorrectly;
* control/input routed to the wrong participant/body;
* coordinate/frame errors;
* stale state crossing lifecycle boundaries;
* simulation depending on presentation state.

### Competing authority

Ask for every important fact:

```text
What owns it?
Who may write it?
What is derived from it?
Who owns its lifetime?
How does rollback restore it?
```

Be suspicious when the same logical fact appears as:

```text
resource
+ component
+ marker saying the component was applied
+ presentation inference
```

Prefer one canonical state with deterministic projections.

### Ownership and lifetime

Identify the owner:

```text
process
application
experience/game
match/session
participant
body/actor
room/world occurrence
simulation tick
presentation frame
```

A shorter-lived owner must not leave its state behind when it exits.

Value equality is not ownership.

### Domain boundaries

A type should describe the domain fact it owns.

Watch for:

* generic relation types accumulating one game's policy;
* render types deciding gameplay eligibility;
* runtime infrastructure knowing leaf-domain types unnecessarily;
* stage/rules objects supplying character capabilities;
* game-specific terminology leaking into generic engine state;
* input plumbing interpreting combat semantics too early.

### Determinism and schedule authority

Important deterministic behavior should follow explicit phases/data dependencies.

Be suspicious when correctness relies on:

* incidental Bevy topological order;
* duplicate registration of the same system in different phases;
* query/event iteration order;
* a later system repairing a fact an earlier system temporarily made invalid.

### Identity distinctions

Do not collapse:

```text
input device
input source
local participant
PlayerSlot
match roster slot
controlled body
AI policy
runtime entity
authored character
persistent occurrence
LocalView
ViewSubject
```

Similarly:

```text
exists
≠ resident
≠ simulated
≠ presented
```

### Coordinate/reference frames

Directional gameplay belongs in the body's resolved semantic frame unless specifically world-authored.

Do not reconstruct one body's local authored geometry from:

* another body's gravity;
* victim-relative knockback direction;
* screen axes;
* camera orientation.

Resolve coordinate ownership where the owning facts are available.

### Authorability

Prefer architecture that changes future work from:

```text
remember hidden registrations
edit several central tables
write game-specific glue
know scheduler trivia
```

to:

```text
author semantic content
validate
run
```

Do not create generic frameworks without a real customer.

## Product-driven architecture

Prefer:

```text
real game requirement
→ missing semantic primitive exposed
→ reusable primitive added
→ game authors it
```

over:

```text
generic abstraction seems useful
→ framework added
→ customer sought afterward
```

A feature can remain game-specific when its semantics are genuinely character/game-specific.

Do not push every successful game mechanic downward into the generic engine.

When multiple domains look similar, prefer semantic convergence before type unification.

## Transitional architecture

Not every temporary seam deserves immediate removal.

Classify it:

### Healthy transitional seam

* bounded clients;
* clear final direction;
* does not attract new policy;
* cheap to replace later.

Leave it unless current work naturally crosses the seam.

### Dangerous transitional seam

* new consumers keep appearing;
* two sources of truth coexist;
* future mechanics must learn host-specific distinctions;
* comments are the only guard preventing expansion.

Prioritize it.

## Abstractions

Do not equate:

* fewer lines with better architecture;
* more crates with better architecture;
* a smaller function with clearer authority;
* more genericity with more reuse;
* queue completion with product quality.

A large function is evidence to inspect, not an automatic refactor request.

Extract around semantic boundaries.

Do not create a function merely to move 400 lines behind another call with the same giant parameter set.

## Simulation / presentation boundary

Simulation owns gameplay truth.

Presentation consumes explicit semantic facts.

Prefer:

```text
simulation state
→ semantic read-model
→ route/game/character presentation policy
→ visual/audio effects
```

Do not:

* let shaders infer gameplay state;
* encode character-specific exceptions in generic renderers;
* collapse composable gameplay reasons into one presentation boolean when presentation needs the reasons;
* make visual effects canonical rollback state unless they affect gameplay.

Presentation effects should be composable when their semantic causes are independent.

## Control architecture

The simulation should operate on ordinary controlled bodies, not a privileged player-body road.

Keep:

```text
AI policy
≠ control authority

control authority
≠ presentation subject

participant
≠ body
```

Human, AI and scripted control should converge before gameplay restrictions/integration whenever possible.

Avoid special participant-0 semantics.

## Body capability doctrine

Ordinary capabilities and inventory belong to the body.

Participant-level entitlements/unlocks are separate.

Possession/control transfer should naturally expose the controlled body's capabilities rather than rewrite body identity.

## Rollback

Rollback state may grow when a genuinely new canonical simulation fact requires it.

Changes must be explicit and schema-versioned.

Do not hide new state inside unrelated already-registered objects merely to avoid changing the schema.

Avoid dynamic `Any`/`TypeId` service-locator rollback registries.

Prefer explicit typed composition.

## Testing

Use the cheapest sufficient test while iterating.

For a defect:

```text
targeted invariant test
+
production-path test when the behavior crosses systems/phases
```

Aggregate simulation tests are evidence about outcomes, not causal proof.

A CPU-vs-CPU distribution changing does not establish which mechanism broke.

When moving shared types across crate boundaries, compile/test the touched packages' test targets.

Follow `AGENTS.md` and current campaign instructions for exact gate commands.

Do not substitute an expensive workspace-wide gate when a narrower gate proves the touched seam.

⛔ A green gate is a claim that the jobs ran, not proof of it. Before trusting
one, ask what the flag skipped, which plan the job is in, whether it re-derived
or reused a cache, and what it would look like if the feature were silently off:
`docs/recipes/checks-that-did-not-run.md`.

## Planning documents

Planning describes current executable direction.

When an explanation is falsified:

* replace the current instruction;
* preserve useful archaeology elsewhere if needed;
* do not leave contradictory paragraphs both looking current.

Prefer queue entries containing:

```text
Observed:
Decision:
Remaining:
Acceptance:
```

Do not turn `queue.md` into a chronological worker transcript.

A detailed old document is not automatically more authoritative than current code.

## Source comments

Production comments should explain:

* invariant;
* owner;
* semantic distinction;
* non-obvious ordering requirement.

Do not preserve:

* investigation chronology;
* failed theories;
* review provenance;
* old test counts;
* arguments aimed at a previous agent.

## Things that require strong evidence before proposing

Do not casually introduce:

* dynamic registries/service locators;
* broad monolith decomposition;
* giant central taxonomies;
* source-text guards for invariants types can encode;
* player-only simulation roads;
* game-specific branches in generic engine kernels;
* global last-writer-wins resources;
* compatibility paths with no removal ceiling;
* positional repair that conceals invalid geometry;
* another source of character capability authority.

## Review pressure tests

Choose pressure tests relevant to the changed subsystem.

Useful recurring cases:

```text
two humans with CPU slot between them
two gamepads, no keyboard
participant changes controlled body
multiple views
same-valued lifecycle handoff between owners
two simultaneous movers
weak external knockback
different gravity/reference frames
rollback through state transition
clean checkout without generated presentation products
same body with two independent grants/effects
```

Do not run every pressure test every review. Select the ones that attack the assumptions of the changed code.

## Finding categories

Use these categories.

### Good / landed correctly

Only substantive wins that should now be left alone.

### Concrete bugs

For each:

```text
severity
file/function
failure scenario
why current tests missed it
correct semantic fix
```

### Architecture debt

Working behavior whose ownership/dependency direction is likely to cost future work.

### Missing acceptance

Implementation may be correct but evidence does not establish the product claim.

### Stale planning

Current prose that would misdirect another agent.

### Healthy transitional seams

Known imperfections that should remain bounded rather than distract current product work.

## Priority output

Do not return forty equal-priority findings.

Prefer:

```text
Fix now
Next architectural slice
Leave alone
Later
```

Choose roughly 3–7 meaningful directions.

Prioritize dependency/order by actual payoff.

## When reviewing a campaign

The campaign document owns temporary product priorities while it is active.

Review it against current code, but do not replace its product goal with a workspace-wide architecture campaign.

A campaign can expose architecture work.

Only pursue that architecture work immediately when:

* the product feature cannot be implemented correctly without it; or
* continuing through the existing seam would create a second authority or durable debt.

Otherwise record the pressure and keep the product campaign moving.

## Reviewer output contract

After inspecting a new snapshot, produce:

### A. Current-state review

* substantive wins;
* concrete regressions;
* architecture debt;
* missing acceptance;
* stale planning.

### B. Priority order

Approximately 3–7 directions.

### C. Steering prompt

A pasteable prompt for the implementation agent containing only current context:

* exact HEAD;
* baseline/delta inspected;
* active campaign/customer;
* concrete current defects;
* intended invariants;
* dangerous shortcuts;
* settled work not to reopen;
* immediate execution order.

Do not copy this evergreen guide into the steering prompt.

The implementation agent should read this file directly.

## Central review question

For every proposed correction ask:

> Does this make the next real feature easier to express through one honest semantic authority, or does it merely relocate today's complexity?
