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

Closed 2026-08-31, in two slices, and carved for OWNERSHIP rather than footprint
(the table above forbids the footprint claim). Four production references
remained after the first slice; two are gone and two are instrumentation:

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
- ▢ **What is left is two `profiling::phase_mark` calls in `audio/plugin`** —
  instrumentation, which can live anywhere, and not an authority the kernel holds.

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
