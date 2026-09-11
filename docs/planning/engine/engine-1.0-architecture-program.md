# Engine 1.0 architecture program

**State:** open, 2D-first programmatic engine with Ambition as flagship customer.
**Review baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`.
This page maps programs and acceptance. [The queue](../queue.md) alone selects
execution; [status](../status.md) reports the current snapshot.

## Product order

Build an engine that can express, inspect, test, iterate and ship substantial
games through programmatic APIs and agent-native authoring. Godot/Unity-class
ambition does not require copying their visual authoring model or claiming
unimplemented 3D/platform support. The near-term product is a reusable 2D engine
whose platformer mechanisms serve the flagship deeply and at least one independent
customer without editing named Ambition engine cases.

The competitive bar includes ordinary capability completeness, sound state and
lifetime authority, supported minimal profiles, measured runtime/build costs,
structured diagnostics, prepared content and distribution. A tidy Cargo graph or
a growing number of techniques is insufficient. See
[Godot-class capability](godot-class-2d-capability.md).

## Current architecture gate

The [reassessment](architecture-reassessment.md) replaces fixed SCC peeling with
local authority claims. Preserve the corrected spawn boundary, typed construction,
confirmed lifecycle commits and existing control/rollback work. Do not globally
label all authority prerequisites complete: checkpoint restoration, target
geometry and several mixed containers still need correction.

Construction, transit and resource hydration are different operations even when
a reset triggers all three. Reconstructing an occurrence should use its canonical
constructor. Relocating an existing body uses the transit authority. Restoring
inventory/resource facts uses its domain hydration authority. Do not force these
into a universal reconstruction enum merely because they share a trigger.

Prepared construction is a plan/preflight/commit/verify/publish protocol. Its
current trusted raw-Commands recipes do not provide rollback of arbitrary world
mutation on verification failure. Preserve no-publication on failure and document
the difference from last-good-world retention. F6/A10 define the conditional
hardening work.

## Capability programs

| Program | Responsibility and acceptance | Owner |
| --- | --- | --- |
| E1: simulation authority, determinism and lifetime | Canonical writers, stable identity, explicit tick/confirmation and correct retirement; rollback across supported transitions | [simulation authority](simulation-authority-and-determinism.md), [instance lifetime](instance-lifetime-provenance-and-persistence.md) |
| E2: construction and reconstitution | Prepared typed domain plans; singular occurrence construction; distinct transit/hydration; stated failure/publication guarantee | [construction](construction-and-reconstitution.md), [immutable content](immutable-content-and-transactional-construction.md) |
| E3: persistent systemic world | Occurrences, custody, durable disposition, nonresident state and real world consequences without hidden duplicate authority | [open world](open-world-runtime-and-residency.md), [items](item-custody-and-accounting.md), [progression](capability-progression-and-world-gating.md) |
| E4: body kernel and domain ownership | A1 checkpoint owner, A2 contacts, A3 placement bridge; conditional control/destructible/character/item moves; no SCC target | [decomposition](actor-monolith-decomposition.md), [frontier](actor-monolith-work-frontier.md), [map](architecture-responsibility-map.md) |
| E5: capability and host composition | Explicit prerequisites, owner installation, semantic cross-owner ordering, absence and re-entry; measured Cargo closure | [composition](capability-and-runtime-composition.md) |
| E6: public SDK | External game needs no internal module map; minimal profiles do useful work and can be packaged | [SDK](public-sdk-1.0.md) |
| E7: runtime, assets and iteration | Scenario/hardware-qualified budgets, correct quality/residency, build/package measurements | [performance](performance-and-iteration.md), [assets](asset-preparation-and-residency.md), [distribution](project-build-and-distribution.md) |
| E8: multiplayer and multiview | Participants, bodies and views stay distinct; real same/different-room customers establish instance scope and transport requirements | [multiplayer](multiplayer-and-multiview.md), [netcode](netcode.md) |
| E9: agent-native authoring | Independent data/module production -> prepare -> test -> admit -> observe; no host relink for normal edits; installed technique and explicit rollback-state contracts | [authoring](authoring-and-tools.md), [world tools](ldtk-authoring-and-world-tools.md), [extension](extension-model.md) |
| E10: world facts and orchestration | Bounded observations and typed domain operations; move-scoped flow stays scoped; runtime model calls are remote intent production | [facts](world-facts-observations-and-memory.md), [orchestration](authored-gameplay-logic-and-orchestration.md), [agentic runtime](agentic-character-runtime.md) |
| E11: presentation and observability | Read models, body-owned drawable geometry, per-view composition, useful structured diagnostics and replay inspection | [presentation](render-animation-and-vfx.md), [inspection](inspection-diagnostics-and-workbench.md) |
| E12: competitive capability | Real authored game slices prove ordinary engine expressiveness and shipping, not a list of implemented nouns | [capability bar](godot-class-2d-capability.md) |

## Cross-program rules

Each operation has one owner for acceptance, mutation and retirement. Shared
vocabulary has semantic meaning; it is not a container for difficult dependencies.
Use direct dependencies when their direction is correct. Introduce registration
only for actual independent providers, not to disguise a closed switch.

Name logical authorities before choosing packages. `features` must dissolve as a
category; `combat`, `characters`, `core` and `runtime` need bounded responsibilities,
not cosmetic renames. Do not move live actor authority back into construction.

Readiness requires both data and implementation support for the selected profile.
A technique whose schema exists but whose runtime handler is absent is not ready.
A flow whose graph indices cannot be represented by its runtime cursor is not
prepared. A missing optional service must not be replaced by a plausible default
that makes a benchmark or gameplay test look meaningful.

Keep product decisions explicit. Q63 still governs ignored authored semantics and
the Interact policy; Q73 is about installer packaging; Q94 is a real hardware/
residency budget. Reviewer recommendations do not become maintainer rulings.

## Program-level exit shape

A small independent game can select a supported headless profile, author and
prepare a world/body/action/object, drive it through semantic input, inspect and
test the outcome, add presentation, and produce a declared platform artifact.
The flagship exercises richer custody, lifecycle, combat and world behavior using
the same authorities. Negative fixtures show unknown references, missing
capabilities, invalid flow/parameters and unsupported target services before play.

Runtime and iteration budgets are measured on the stated hardware/scenario/cache
conditions. Replays identify the build, prepared revision, input trace and profile.
No unsupported source-only claim substitutes for GPU, P2P or platform evidence.

## Open program questions

Public cross-version save compatibility, untrusted mod distribution, concurrent
world scheduling and target hardware promises remain explicit product decisions.
The present compile/iteration requirement already justifies the
[extension architecture](extension-model.md). Its dependency-light data and
procedural boundary does not wait for those product choices. Backend and snapshot
storage choices use the named measurements, not a maintainer poll. Do not turn
this into a universal world identifier, backend-neutral engine rewrite or parity
campaign. Existing domain correctness work retains its priority.
