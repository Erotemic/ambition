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

### What the host boundary actually enforces

Stated precisely, because prose elsewhere has claimed more than the tests check:

- **Enforced:** `ambition_host` may not depend on or name `ambition_content`
  (policies `engine.host-names-no-content`, `engine.host-source-names-no-content`).
- **Now enforced (since the July-18 policy update; this paragraph previously
  said otherwise):** the host may not *directly* depend on or name
  `ambition_actors` — `engine.host-manifest-no-actors` and
  `engine.host-source-no-actors` (`tests/ambition_workspace_policy/policies/engine.toml`).
  Historical context: F1.10 (2026-07-07) removed the direct dependency but its
  claimed boundary test only ever checked `ambition_content`; the actors guard
  arrived later. `ambition_render` carries the analogous guards
  (`engine.render-no-actor-crate-dependency`,
  `engine.render-source-names-no-actors`).
- **Already true in practice:** the host reaches `ambition_actors` transitively
  through `ambition_runtime`, and has since E5 step 5 (2026-07-06).

So the DIRECT edge is now a ratchet; the *transitive* reach (host →
`ambition_runtime` → actors) remains real and intentional. Do not quietly
widen the guard to transitive naming — see below.

### Unsettled: where device input lives when it needs sim vocabulary

**Uncertain — do not treat either answer as doctrine yet.**

The concrete case is touch/pointer input. `ambition_touch_input`'s overlay is
window/device wiring by nature, which argues for `PlatformerHostPlugins` (that is
literally what the group is: the windowed host's camera + input). Composing it
there is what makes every game — Ambition, the demos, anything added later — get
touch by construction instead of each app remembering. Today only
`ambition_app` composes it, so the standalone demo apps have no touch at all.

What blocks a clean answer is that the overlay names `ambition_actors` for
`affordances::{ActiveInputMethod, glyph_for, AffordancesSystemSet}`, the input
schedule labels in `schedule::input_systems`, and `control::populate_slot_controls`.
Some of its other actors imports are *already* compat re-exports of foundation
types (`PrimaryPlayer`, `GravityField`, `gravity_dir_or_default` all live in
`ambition_platformer_primitives`), so the real coupling is narrower than the
import list suggests — but it is not zero.

**Preferred direction: invert rather than ban or shrug.** Layers should be able
to communicate at RUNTIME — shared read-models, published seams, schedule sets
owned by the layer that defines the ordering contract — without a compile-time
dependency pointing the wrong way. `ControlPrompt` is the model to copy: the
touch overlay reads the controlled subject's live scheme from
`ambition_sim_view` and never reaches into the sim heart, so possessing a
different body relabels the buttons with no per-game code. The remaining actors
imports are the ones that have not had that treatment yet.

Neither "put it in the host and accept the dependency" nor "forbid it until the
inversion is finished" is ruled correct here. Whoever resolves this should first
repoint the compat re-exports at their canonical homes and re-measure what is
actually left, then decide against that list rather than against this paragraph.

**Worked precedent (2026-07-19).** The shell pause menu and launcher read
devices directly (`ButtonInput<KeyCode>` + `Query<&Gamepad>`), so the touch HUD's
"Menu" button reached no shell surface and an Android session had no way back to
the title screen. The fix was the inversion, not a new dependency on the input
stack: `shell_action_edges` now also folds `MenuControlFrame` — the neutral
menu-intent resource every OTHER menu already consumes, which touch, mouse wheel,
and on-screen buttons write into. `ambition_input` is a leaf (bevy only, no
`ambition_*` deps), so the shell gained one downward compile edge to a vocabulary
crate and zero knowledge of any device. The resource is read as `Option<Res<_>>`
and both sources carry one-frame edges, so the shell needs no schedule set owned
by a crate above it. That combination — vocabulary crate below, runtime seam,
order-independent by construction — is what "invert rather than ban" looks like
in practice, and is the shape to try first on the touch-overlay question above.

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
