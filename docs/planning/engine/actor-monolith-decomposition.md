# Actor residual-kernel decomposition

**State:** OPEN — incremental ownership/dependency work.

Durable decomposition doctrine:
[`../../architecture/package-and-capability-boundaries.md`](../../architecture/package-and-capability-boundaries.md).
Capability closure:
[`capability-and-runtime-composition.md`](capability-and-runtime-composition.md).

## Goal

Reduce `ambition_platformer2d_actor_monolith` to an honest reusable actor/body
simulation kernel. The objective is **not** a target line count and is **not** a
promised frame-time win.

The current reasons to continue are:

- architectural ownership;
- dependency direction and capability closure;
- compile/change isolation;
- test isolation;
- safer feature work;
- reusable engine packages;
- a public SDK that does not expose historical implementation topology.

Recent runtime measurement found a broad set of individually small systems and
did not establish monolith decomposition as a frame-time optimization. Treat
runtime savings as evidence to measure, not as the campaign rationale.

## Residual kernel target

The final actor package should own the closely coupled simulation that truly
belongs to an actor/body:

- authoritative actor/body state;
- control/intent acceptance and actor-local decision integration;
- movement/contact/body-mode integration;
- actor-local lifecycle semantics;
- narrow action/body integration interfaces;
- construction needed specifically to establish the actor-local simulation
  state above.

It should not retain a concern merely because actors happen to use it.

Strong candidates to live outside the residual kernel include:

- named provider/game content and catalog lookup;
- dialogue/conversation;
- encounters and boss orchestration;
- persistence/session policy;
- menus/UI;
- audio/VFX/presentation;
- developer facilities;
- optional item/projectile/portal/mount capabilities with independent semantics;
- world-authoring/backend integration;
- host/platform composition.

Some of these already have dedicated owners. Prefer completing those ownership
moves over inventing new crates.

## Current measured dependency shape

The last reconciled census on 2026-08-29 distinguished three different numbers:

| measure | value |
|---|---:|
| `ambition_*` lines in the monolith `[dependencies]` table | 28 |
| direct edges in the default resolved graph | 27 |
| full default resolved closure | 34 |

Do not compare these as though they were the same metric. Before a footprint
carve, run `cargo tree -i <dependency>`; another path may already keep the crate
in the product closure.

⛔⛔ **THE "SINGLE PATH" LIST IS STALE — RE-MEASURED 2026-08-31.** The census
named four monolith dependencies as reaching the closure through one path and
therefore able to shrink it:

| dependency | dependent crates today |
|---|---:|
| `ambition_dev_tools` | 6 |
| `ambition_mount` | 6 |
| `ambition_items` | 5 |
| `ambition_damage` | 3 |

None is single-path. And the stronger statement, which retires the rationale
rather than just the number: **`ambition_platformer2d` — the facade every game
depends on — depends on all four directly** (`items` optionally). So removing the
MONOLITH's edge to any of them cannot shrink a product's closure at all, however
the census is counted.

⭐ **SO FOOTPRINT IS NOT A REASON TO CARVE THESE FOUR.** The remaining reasons are
the ones this doc already leads with — ownership, dependency direction, compile
and test isolation — and a slice that promises closure movement for one of them
is promising something arithmetic forbids. Re-run
`grep -rl "^<dep>" crates/*/Cargo.toml game/*/Cargo.toml` before citing any
single-path claim; it is cheaper than `cargo tree` and it is what went stale.

### ✔ `ambition_dev_tools`: the kernel no longer reads or writes developer state

Closed 2026-09-02, in four slices, and carved for OWNERSHIP rather than
footprint (the table above forbids the footprint claim). ⚠ THE COUNT OF WHAT WAS
LEFT WAS RESTATED THREE TIMES AND WRONG TWICE — read the trail below rather than
any one of its numbers, because each was written as if it were the last. The
kernel's production `[dependencies]` edge to this crate is gone; the
`[dev-dependency]` the `#[cfg(test)]` fixtures use is not, and is not meant to
be. What follows is in the order the references fell:

- **the WRITE.** `cleanup_timers_system` decayed `dev_state.preset_flash`, a
  developer HUD timer, and that one line was the only reason the control module
  held a `ResMut<DeveloperRuntimeState>`. Now
  `ambition_dev_tools::decay_developer_presentation_flash`, in the SAME schedule
  — its old home ran in `PresentationSync` so presentation timers decay while
  gameplay is suspended, and `Update` counts a different clock under rollback.
- **the READ.** `update_time_scale_requests` read `dev_state.slowmo` at rung 4 of
  a five-rung ladder, twice. ⭐ THE INVERSION WAS ALREADY BUILT AND UNUSED:
  `ClockRequester::DevTool` exists, `RegimePolicy` grants it in `Solo` and denies
  it in `RLDeterministic`/`Cinematic`, and `apply_clock_scale_requests` reduces by
  `min`. The dev crate publishes its own `ClockScaleRequest`; the ladder lost both
  rungs. ⚠ `min` IS NOT A LADDER: bullet-time used to outrank slow-motion by
  sitting at rung 2, and now the stronger slowdown wins. Stated, not hidden.
- `debug_slowmo_scale` moved with the rung. It was a developer number in
  `Platformer2dFeelTuningMonolith`, whose own module doc says those values *"are
  gameplay parameters rather than developer-tool state"*, and nothing but that
  rung read it.
- ✔ **The two `profiling::phase_mark` calls in `audio/plugin` are gone
  (2026-09-02).** Instrumentation can live anywhere, and a simulation crate
  naming a profiler to describe itself is the thing this carve is about. The
  plugin publishes `AudioInitSet`; the host brackets it in `app/plugins.rs`
  beside every other `phase_mark`, under the same two names so existing startup
  profiles still compare.
- ⛔⛔ **BUT "WHAT IS LEFT" WAS WRONG, and the carve is not one step from done.**
  Counted 2026-09-02: the monolith still reads `ambition_dev_tools` from THREE
  more production paths, and they are not instrumentation — they are the
  simulation reading developer state, which is exactly what this carve exists to
  remove:
  - ✔ `features/npcs.rs` — CLOSED 2026-09-02. `forced_profile()` / `forced_preset()`
    were chosen while building a live brain; the sim reads a session-owned
    `AuthoredBrainOverride` the dev tool writes, the two `OnceLock`s are deleted,
    and `for_room_construction` takes the authority as a parameter so no road can
    forget it;
  - ✔ `features/ecs/spawn_static.rs` — CLOSED 2026-09-02 evening. The quota is
    `ActorAdmission` spent at plan time in `prepare` (a refused NPC plans no row), the
    value is a published `AuthoredPopulationCap`, `for_room_construction` takes
    it as a parameter, and the dev crate keeps only the env name and parse;
  - ✔ `features/mod.rs:350` — `runtime_census` — CLOSED 2026-09-02, and ⛔ THE
    COUNT IN THIS PARAGRAPH WAS ALSO WRONG. "THREE more production paths … they
    are not instrumentation" undercounted by one and mis-described the
    remainder: after the two above closed, the survivors were **TWO** and BOTH
    were instruments. The uncounted one was `features/ecs/actors/update.rs:606`,
    `perception_census::note_world_view`, called inside the `build_world_view`
    loop — a hot path, which is why it could not simply be observed from
    outside.
  - ✔ `features/ecs/actors/update.rs:606` — CLOSED with it, the same day.

⭐⭐ **AND THE EDGE IS GONE: `ambition_dev_tools` IS A `[dev-dependency]` OF THE
KERNEL NOW.** Both instruments moved by publishing the thing DOWNWARD, which is
the third option beside the two this doc had been treating as exhaustive (delete
the measurement, or move `phase_mark` down) — the same inversion `AudioInitSet`
made one bullet up:

- `ActorDecisionSet` moved from `pub(crate)` in `features/mod.rs` to
  `shared_tangle::schedule`, beside `WorldPrepSet` and `PlayerInputSet` — which
  the census-boundary function was ALREADY importing from there, so the enum was
  the only reason the marks could not live with the instrument.
  `runtime_census::install_sim_phase_boundaries` now installs all seven beside
  its other twenty, under the same `if enabled` gate and the same "an instrument
  must not join the population it measures" rule. ⛔ The kernel still CONFIGURES
  the chain: `configure_actor_decision_phases` and every ordering assertion in
  `actor_decision_phase_tests` stayed, because WHERE the sets sit is the
  simulation's business. What left is the developer registration.
- `perception_census` (91 lines) moved to
  `ambition_characters::perception::census`, the module that defines the
  `WorldView` it counts. The developer crate still calls `enable` where it
  installs its rows and `drain` where it prints them, so the POLICY did not
  move — only the counter.

⛔ **THE GUARD IS THE MANIFEST, not a test, so it cannot rot.** A production
`use ambition_dev_tools::…` anywhere in the kernel's `src/` no longer compiles.
Poison-verified by adding one: `error[E0433]: cannot find module or crate
`ambition_dev_tools` in this scope`. The `#[cfg(test)]` live-refresh and reset
tests keep reaching it, which is what a dev-dependency is for.

⚠ **AND IT BUYS NO SMALLER CLOSURE, which the table above already forbade
claiming.** `ambition_dev_tools` has 6 dependents and the facade names it
directly; nothing downstream drops it. The payoff is ownership and a boundary
the compiler holds.

⭐ **THE COST WAS ONE POLICY LINE, and it is the honest kind.**
`engine.ambition_dev_tools-manifest-allow` gained `ambition_time`, with the
reason in its rationale: the dep exists so the SIMULATION stops reading developer
state, and `ambition_time` depends only on `ambition_platformer2d_core`, so it is
foundation rather than sim. ⛔ a carve that needs an allowlist widened should say
what the widening buys, in the allowlist.

⛔ **AND REGISTRATION NEEDED AN APP-LEVEL GUARD BOTH TIMES.** A moved system's
behaviour can be proven in its new crate; that anything RUNS it cannot be —
`DevToolsSimPlugin`'s siblings need resources `ambition_dev_tools` does not
depend on, so its schedule will not run in a bare `App`, and bevy names every
system `<Enable the debug feature to see the name>` without a feature that crate
will not enable for a test. Both guards live in `app_it` and both go red when the
system is unregistered.

## What recent carves taught

Several completed slices established rules that should guide future ones:

- A forwarding/re-export edge can be worth deleting even when total closure does
  not move; it makes ownership honest and prevents future callers from learning
  the wrong path.
- Gating one obvious dependency does not help footprint when another carved
  domain re-supplies it. Count all suppliers first.
- A domain should publish a semantic event/fact and let an optional presentation
  or orchestration consumer translate it rather than importing the optional
  domain downward.
- Domain-owned rollback registration is no longer a reason for generic runtime
  or actor packages to know each concrete gameplay component.
- Moving files without moving the authority, plugin registration and dependency
  edge is not a carve.

The detailed sequence of LDtk, conversation, boss, mount and other historical
carves is available in git history and should not be reconstructed here.

## Slice selection

Before starting a carve, answer:

```text
What authority is moving?
Who owns it after the move?
What systems/resources/messages move with it?
What dependency edge or change-fanout path should disappear?
Does another path keep the dependency in product closure?
What small App or consumer can test the new owner?
What old facade/import/registration can be deleted?
```

A good slice normally satisfies more than one of:

1. removes a direct monolith dependency;
2. shrinks a minimal consumer's resolved capability footprint;
3. removes duplicate/shared authority;
4. isolates a meaningful compile/test unit;
5. deletes a compatibility/re-export path;
6. creates a coherent domain plugin with an independent test surface.

## Current frontier

Prefer outside-in ownership work before splitting the central actor kernel.
Useful frontiers are:

### Developer and presentation dependencies

Developer tools and presentation facilities are poor reasons for the simulation
kernel to depend upward. Remove them when the consumer can observe semantic
facts through an existing engine seam.

⭐⭐ **RE-MEASURED 2026-08-31, AND THE PRESENTATION HALF OF THIS FRONTIER IS
NARROWER THAN IT READS.** The kernel's production references to presentation-ish
crates are:

| crate | refs | files | what it is |
|---|---:|---:|---|
| `ambition_sfx` | 160 | 35 | a cue MESSAGE vocabulary + writer, 1 ambition dep |
| `ambition_vfx` | 103 | 31 | the same shape, 3 deps |
| `ambition_audio` | 70 | 7 | channel/registry types, 2 deps |
| `ambition_conversation` | 33 | 9 | 8 deps |
| `ambition_cutscene` | 16 | 2 | script + stepper, 1 dep |

⛔ **NONE OF THEM PULLS A RENDERER.** No `bevy_render`, `bevy_audio`, `bevy_ui`,
`bevy_sprite` or `bevy_text` anywhere in that column, so the closure argument
that would justify carving them does not exist — they are FOUNDATION vocabulary
crates the kernel consumes DOWNWARD, and a body emitting its own hit cue is the
semantic fact, not a presentation reach.

⚠ `ambition_dialog` and `ambition_sim_view` are DEV-DEPENDENCIES here, so they
are not production edges at all; a count that read the manifest without its
section headings would report two edges that do not exist.

⛔ **AND `cutscene.rs` ALREADY CARRIES ITS OWN DEFENCE**, which a carve should
answer rather than repeat: *"These systems are gameplay-coupled (rooms, save,
schedule) so they live here rather than in `ambition_cutscene` — which sits below
this crate and must stay content- and gameplay-free."* Moving it needs a THIRD
crate above both, which is a cost with no measured benefit behind it.

⇒ **The developer half of this frontier is done and the presentation half is
mostly a mirage.** Pick the next slice from the domains below, not from here.

### Items, mounts and other optional gameplay domains

Move vertically: the domain owns its state/messages/plugin, the actor kernel
exposes the body/action hooks it needs, and the old actor-side implementation is
deleted. Do not replace one central switch with another.

⭐⭐ **AND THE ITEMS HALF IS DEFENDED TOO — measured 2026-08-31 before opening
it.** `src/items/` is 6580 lines against `ambition_items`' 1975, which reads like
a domain sitting in the wrong crate. It is not:

* 1864 of those lines are `pickup/tests.rs`, plus ~270 more in sibling test
  modules — about **4.4k lines of production code**, not 6.6k;
* `ambition_items` already owns what the doc asks a domain to own — the 24-slot
  catalog, owned-item state, the shop, and the `item_catalog` content schema;
* the kernel's half says what it is in its own first paragraph: *"The
  pickup/throw/projectile steppers stay here because they mutate actor bodies,
  gravity, portals, abilities, and hit events."*

⇒ **a stepper that mutates bodies belongs with bodies.** Moving it is not a move:
this frontier's own instruction — *"the domain owns its state/messages/plugin,
the actor kernel exposes the body/action hooks it needs"* — means DESIGNING those
hooks first, generic enough that an item crate can drive a pickup without naming
gravity, portals, abilities and hit events. That is a design slice with a real
budget, and nothing above measures it.

⛔ **So three of this doc's frontiers have now been checked and only one was
work.** Developer dependencies: done. Presentation: no renderer anywhere, a
mirage. Items: defended, and a carve needs a hook design first. A future slice
should start from the SEAM it intends to add, not from a line count — the top of
this document already says a line count is a proxy, and these three are what that
warning looks like in practice.

### Encounter/conversation/world orchestration

The actor kernel should emit/consume small semantic facts. Room, encounter,
conversation and provider orchestration belongs to their owning runtime/domain
packages.

### Character preparation versus actor simulation

Prepared character/content ownership should continue moving toward character
and provider packages. The residual actor kernel consumes prepared body/action
facts rather than becoming the content compiler/catalog.

### Central kernel split

Do this last. Once outer domains are gone, the remaining dependency graph will
show whether body state, movement, decision integration and construction still
need one crate or have another stable seam. Do not pre-split this core because a
source file is large.

## Explicit non-goals

Do not:

- carve by LOC targets;
- add wrapper crates that import the whole monolith;
- scatter feature gates through the kernel merely to move a `cargo tree` number;
- duplicate runtime authority during migration;
- keep historical re-exports for compatibility in this pre-release engine;
- claim runtime/startup improvement without an A/B measurement;
- turn every internal domain into an independently published Bevy crate.

## Exit

This program is complete when:

1. the residual actor package can be described as actor/body simulation without
   listing unrelated product capabilities;
2. optional domains install through semantic capability/plugin seams rather than
   actor-kernel imports;
3. major game/provider orchestration no longer lives in the actor package;
4. minimal consumers do not inherit unrelated domains through the residual
   kernel;
5. the public facade exposes actor semantics without exposing the historical
   monolith name/topology;
6. remaining dependencies are justified by genuine actor-kernel ownership, not
   migration residue.
