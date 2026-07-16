# Decomposition doctrine

This is the current architectural doctrine, not the old carve-by-carve ledger.
The measured 2026-07-09 through 2026-07-15 execution history is archived at
[`docs/archive/reviews/decomposition-ledger-through-2026-07-15.md`](../../archive/reviews/decomposition-ledger-through-2026-07-15.md).
Current accepted work is in [`../tracks.md`](../tracks.md).

## What earns a crate

A crate split must create a durable semantic boundary with at least one concrete
benefit: a lower dependency surface, an independently usable engine face, an
independent owner/test surface, or elimination of a misleading composition
boundary. Size alone is not sufficient.

Prefer internal modules when code shares runtime authority, schedule ordering,
private invariants, and nearly all consumers. Do not replace one understandable
crate with a chain of forwarding facades or abstract service traits.

## Settled ruling: no size-driven `ambition_actors` carve

The post-carve actor crate is the authority-woven simulation adapter around one
body/control/motion path. Its remaining mass is spread across actor spawning,
perception, control, body integration, boss policy, world/contact adapters,
projectile victim routing, presentation publication, and content hooks. Splitting
those by LOC would risk recreating player/enemy/boss paths and does not produce a
clean independent consumer.

This ruling does not protect misplaced named content or prevent a later split
that a real second consumer demonstrates. In particular, boss decomposition is
reassessed only after boss execution converges onto the canonical moveset path.

## E4 — one-way observation boundary

Simulation owns authoritative mutable state. `ambition_sim_view` publishes
stable read models for render, headless agents, replay/netcode confirmation, and
observer-relative presentation. Presentation does not mutate simulation and the
simulation does not depend on its renderer.

Do not copy immutable authored world IR into `SimView` merely to reduce an upper
layer's dependency count. Add a view projection when it protects mutable truth,
observer policy, deterministic serialization, or a replaceable consumer.

## E5 — runtime and host faces

`ambition_runtime` is the headless simulation assembly. It owns the global phase
ordering contract and composes domain plugins/sets. Domain crates own their local
messages, resources, systems, and schedule sets.

`ambition_host` owns window/device/presentation wiring. It does not become a
second simulation assembly and must not depend directly on actor implementation
internals.

The accepted additional engine faces are:

- a dedicated platformer-provider lifecycle crate, extracted from the umbrella facade;
- `ambition_sim_harness`, the programmatic reset/step/action/observation surface.

## F1.5 — simulation and presentation stay separated

`ambition_render` is downstream of simulation and read models.
`ambition_actors` never imports render. Content may provide presentation plugins
through public render seams, but named game art/modules do not live in the
default renderer merely because they draw sprites.

## W3 — authored world IR and lowering

`ambition_world` owns backend-neutral room/space IR, the closed common Tier-0
placement schema, room graph, placement records, moving-platform math, and the
composited collision read API. Authoring backends convert into this IR.

Lowering is the canonical translation from typed authored placement IR into live
session-scoped ECS representation. Activation, reset, transition, and restore
must all use the App-installed lowering registry. A provider-specific authored
placement channel, if ever needed, is separate from and does not reopen the
closed common schema.

## E-enc — encounter ownership

`ambition_encounter` owns reusable encounter state, participants, objectives,
gates, and lifecycle vocabulary. Actor/content adapters execute body-specific or
named behavior above it. Encounter is orchestration, not an actor type.

Cutscenes remain a separate domain: they are scripted with limited interaction;
encounters are interactive with limited scripting.

## F1.10 — windowed host isolation

The host composes input, presentation, runtime, loading, and shell behavior
through public engine faces. It does not name `ambition_actors` internals or own
provider-specific gameplay lifecycle branches.

Explicit provider plugin registration in the composition root is intentional;
opaque discovery is not a goal.

## Navigability below crate boundaries

- One module, one named concern.
- Split a file when it becomes hard to search or reason about, not to satisfy an arbitrary small number.
- Production modules above the repository's generous review threshold receive deliberate review, not automatic fragmentation.
- Each crate's `MODULES.md` states ownership and composition, and must track real source layout.
- Delete migration facades and duplicate paths when the universal path lands.

## Current decomposition work

Only the current queue in [`../tracks.md`](../tracks.md) is active. Completed
carve IDs, old LOC projections, compile-time samples, and execution task cards
are historical evidence and must not be reintroduced here.
