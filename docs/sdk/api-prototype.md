# The host-composition API, as call sites

**Slice A2 of [the API 1.0 campaign](../planning/engine/api-1.0-campaign.md).
No implementation. Judged by reading.**

The rule this document exists to obey is
[ADR 0031](../adr/0031-public-facade-is-the-compatibility-boundary.md)'s
sequencing: *the public module names are a consequence of what the call sites
need, not an input to them.* So the call sites are written first, here, and the
module list at the bottom is read off them.

**Bounded to host composition.** No content model, no character authority, no
capability staging, no rollback federation. Those are slices B–D, and A2 is the
row most likely to quietly absorb them — the campaign's own first draft did
exactly that and had to be split.

---

## 1. What a consumer writes today

`fixtures/external_consumer/src/lib.rs`, `build_windowed_app`: 64 lines, of
which these are **engine rules the consumer had to re-derive**. Three are
recorded in that function's own comments as leaks found the hard way; the rest
were never written down at all.

| # | Rule | Fails how? |
|---|---|---|
| 1 | the consumer's asset source registers **before** `DefaultPlugins` — Bevy seals asset sources when `AssetPlugin` builds | silently: assets resolve against the engine tree |
| 2 | `AssetPlugin.file_path` must be `asset_manager::actors_desktop_asset_root()` | silently: engine content does not load |
| 3 | headless needs exactly five disables (`LogPlugin`, `TerminalCtrlCHandlerPlugin`, `CorePipelinePlugin`, `GizmoRenderPlugin`, `WinitPlugin`) plus `RenderPlugin { backends: None }` | loudly, eventually, per plugin |
| 4 | `init_engine_states` before the engine plugin groups | resource-missing panic |
| 5 | `PlatformerEnginePlugins` before `PlatformerHostPlugins` before the shell | resource-missing panic |
| 6 | `PlatformerAssetsPlugin` **after** the content that registers the catalogs it reads and **before** the presentation that draws what it installs | silently: unskinned bodies |
| 7 | a host that never names an initial route prepares and activates **nothing** | silently: an earlier draft of the headless binary "ran" 120 ticks of an empty host |
| 8 | manual stepping needs `TimeUpdateStrategy::ManualDuration(Time<Fixed>::timestep())`, read back out of the world *after* the plugins built it | silently: frame dt drifts from tick dt |

Four of the eight fail **silently**, which
[§3a of the growth method](../planning/engine/api-growth-method.md) prices at
triple. That is the leak slice A closes: *a rule the engine knows and makes the
consumer re-derive.*

---

## 2. The call sites

### 2a. A minimal visible game

```rust
use ambition::app::prelude::*;

fn main() {
    PlatformerApp::windowed("Outlander — external consumer proof")
        .mount(OutlanderModule::default())
        .run();
}
```

That is the whole `main`. Every rule in §1 is inside it.

### 2b. The same game, headless

```rust
use ambition::app::prelude::*;

let mut app = PlatformerApp::headless()
    .mount(OutlanderModule::default())
    .build();

app.update();   // exactly one sim tick — rule 8 is already applied
```

`headless()` and `windowed(title)` are the same composition in the same order,
selected by one call.

> ⚠ **Corrected 2026-07-30 — this said they "differ in policy, not in structure:
> same modules, same order, same routes", and the last clause is false.** The
> visible face installs `PlatformerAssetsPlugin`, which panics without a
> `CharacterCatalog`, so **a module that boots headless does not necessarily
> boot windowed.** The blind agent hit it and recorded that the document had
> "actively told me the opposite would be true", which is the most expensive
> kind of wrong.
>
> The underlying divergence is deliberate — see §4 on `with_game_assets` — but
> its consequence for a *minimal* module was written down nowhere. Closing it
> needs an empty-content story, which is slice B; until then this asymmetry is
> a stated limitation rather than a surprise.

The GPU-less variant — a real render graph against no wgpu backend, for CI — is
a third policy on the same axis, not a fourth composition:

```rust
PlatformerApp::windowed("…").without_gpu().build()
```

### 2c. A studio that already owns its `App`

ADR 0031 decision 5: *`PlatformerApp` is a Bevy plugin group, not a runtime. A
studio with an existing Bevy `App` must be able to add it without surrendering
the `App`.*

```rust
let mut app = App::new();
app.add_plugins(TheirOwnInspector);
app.add_plugins(
    PlatformerApp::headless()
        .mount(OutlanderModule::default())
        .as_plugin_group(),
);
```

> ⚠ **This form cannot honor rule 1, and saying so is the point.**
>
> Asset sources must be registered before `AssetPlugin` builds. A plugin group
> added to an `App` that already has `DefaultPlugins` is *already too late*, and
> no amount of ordering inside the group fixes it. The engine owns ordering
> **within its own installation**, not the consumer's process.
>
> So the declaration carries its asset sources as **data** (§3), which lets the
> engine do the one thing it can still do: **detect the violation and say so**.
> `as_plugin_group()` checks whether `AssetPlugin` has already built with a
> declared source unregistered, and fails with a structured diagnostic naming
> the source and the fix.
>
> That converts §1's rule 1 from a silent failure into a loud one. It does not
> close the leak — it prices it correctly, which is what the growth method asks
> for. A rule the engine cannot own must at least be a rule the engine
> *enforces*.

---

## 3. The module declaration

The smallest thing host assembly needs, and no more.

```rust
#[derive(Default)]
pub struct OutlanderModule;

impl GameModule for OutlanderModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest::new("outlander")
            .asset_source(AssetSource::at("outlander", outlander_asset_root()))
    }

    fn define(&self, module: &mut ModuleDraft) {
        module
            .experience(OUTLANDER_EXPERIENCE)
            .launcher_route(OUTLANDER_LAUNCHER_ROUTE)
            .gameplay_route(OUTLANDER_GAMEPLAY_ROUTE)
            .room(outlander_room().metadata)
            .capability(OutlanderExperiencePlugin);
    }
}
```

**`&self` on both methods, per campaign §A2.** Not because `Box<dyn GameModule>`
demands it — generic `mount(SanicModule { difficulty })` erased into a prepared
value is sufficient — but because a receiver-less `define` or an associated
`const ID` forecloses parameterised modules for nothing:

```rust
PlatformerApp::windowed("Sanic").mount(SanicModule { difficulty: Hard })
```

**`define` mutates a draft, never an `App`** — [ADR 0032](../adr/0032-authoring-is-declarative.md)
decision 1. Nothing a module writes is live when `define` returns. In slice A
the draft holds only what host assembly consumes (routes, room metadata, the
experience capability); `ContentPackDraft` and everything under it is **slice
B**, and A must not grow a content method it cannot yet validate.

**`manifest` is separate from `define` because it is needed EARLIER.** Asset
sources have to be installed before `DefaultPlugins`; routes and capabilities do
not. Two methods is the honest expression of two lifecycle stages — folding them
into one would put the asset source behind the same barrier as the content and
reintroduce rule 1 from the inside.

### `run()` needs no `start_at`

`OUTLANDER_GAMEPLAY_ROUTE` is declared *in the module*, so rule 7 — the
never-named initial route that boots a router pointing nowhere — is not
something a consumer can forget. If a module declares no gameplay route,
`mount` fails at declaration time with one error, not at frame 120 with silence.

---

## 4. Sessions: fixed-step only in slice A

```rust
pub enum SessionMode { FixedStep }
```

`PlatformerApp` exposes fixed-step and nothing else. **Rollback is deliberately
not a public knob in A** — the campaign's Deferred section lists why, and it is
not a clock: frozen schema, complete authoritative baseline, stable
participants, deterministic activation, lifecycle rebasing, confirmation
boundaries. Its own slice, its own acceptance tests.

> ⚠ **The one place slice A risks ending with two paths.**
>
> `fixtures/external_consumer` has a THIRD composition —
> `build_outlander_rollback_app` — which differs from the headless one by
> `PlatformerEnginePlugins::rollback()` in place of `fixed_tick()`, plus
> activation before session start. If A4 migrates windowed and headless and
> leaves that one hand-composed, the fixture ends the slice with a facade path
> and a raw path, which is rule 4's exact violation.
>
> **Proposed resolution, to be accepted or overruled before A3:** the builder is
> the single composition authority and the rollback host is a *variation* on it,
> reached through `#[doc(hidden)] fn unstable_rollback_session()` that carries
> the deferral in its own doc comment. One authority, one publicly supported
> mode, no second composition. The alternative — leaving the rollback app on
> raw paths — is a fork, and calling it "deferred" would not make it one.
>
> Cost of the resolution: `ambition::runtime::rollback` stays in the allowlist
> baseline through slice A. That is correct and should not be hidden; the
> ratchet is supposed to still show it.

---

## 5. The module list, read off the call sites

Per ADR 0031, this is an **output**. Everything §2 and §3 name:

```text
ambition::app          PlatformerApp, SessionMode, AssetSource
ambition::experience   GameModule, ModuleManifest, ModuleDraft
ambition::app::prelude the two above, and nothing else
```

Three modules, one prelude, for the whole of host composition.

**Domain preludes, not one root prelude** (campaign §A2). `ambition::app::prelude`
carries what a `main` needs. `ambition::world::prelude` and
`ambition::character::prelude` are slice-B surfaces and are not invented here.
One enormous root prelude is a discovery problem for an agent, not a
convenience — an agent that imports 300 names has been told nothing about which
40 matter.

### Projected effect on the A1 ratchet

Outlander names 18 top-level modules today. Five are already semantic surfaces
(`engine`, `windowed_host`, `presentation`, `game_assets`, `provider`); thirteen
are raw crate mirrors. Host composition is what §2 and §3 replace, so the
modules slice A can retire are the ones named **only** for composition:

A module retires only if **every** name the consumer reaches through it is
composition. Measured, not assumed — and measuring it moved two rows:

| Module | What the consumer names through it | Retired by A4? |
|---|---|---|
| `windowed_host` | `PlatformerHostPlugins` ×4 | **yes** — composition only |
| `game_assets` | `PlatformerAssetsPlugin` ×4 | **yes** — composition only |
| `presentation` | `PlatformerPresentationPlugin` ×1 | **yes** — composition only |
| `engine` | `PlatformerEnginePlugins::fixed_tick` ×4, `add_headless_foundation` ×3, `init_engine_states` ×2, `PlatformerEnginePlugins::rollback` ×1 | **conditional** — composition only, but the last one retires only if §4's resolution is accepted |
| `provider` | `ShellComposition` ×1, **`PlatformerAuthoredCatalogRegistry` ×1** | **no** — the second is content (slice B) |
| `asset_manager` | `actors_desktop_asset_root` ×5, `consumer_source::layered_asset_source` ×1, **`sandbox_assets::SandboxAssetCatalog` ×1, `sandbox_assets::ids::sfx_bank` ×1** | **no** — composition closes six of eight uses; the asset *catalog* is slice B |
| `runtime` | `SIM_TICK_HZ`, `rollback::*` | no — rollback, deferred (§4) |
| `actors`, `characters`, `world`, `entity_catalog`, `sprite_sheet`, `input`, `time`, `audio`, `engine_core`, `platformer`, `game_shell` | content, gameplay, vocabulary | no — slices B+ |

**Predicted: 18 → 14.**

> The first draft of this table said **12**, retiring `provider` and
> `asset_manager` on the strength of their composition uses without checking
> whether they had others. They do. This is the whole reason the prediction is
> written down *before* A4 runs and checked against the instrument afterwards:
> a number derived from what a slice is *about* is a story, and a slice that
> reports 14 against a remembered guess of 12 has learned nothing, while one
> that reports 14 against a recorded 14 has confirmed its model of its own
> consumer.

Note what the two corrected rows say about the API: **six of `asset_manager`'s
eight uses close, and the module stays.** Module-granularity is the ratchet's
unit and it is a *coarse* one — it will report progress late and understate it.
That is the right direction for a gate to be wrong in, but §2a of the growth
method should be read with the per-path counts above, not with the module count
alone.

---

## 6. Explicitly NOT in slice A

Listed because the campaign's first draft absorbed all of it:

* content as a value — `ContentPackDraft`, pack identity, namespaces, content
  fingerprints, `UnresolvedContentRef` → `ResolvedContentRef` (**B**);
* character authority, facet schemas, the `PreStartup` backstop deletion (**B** —
  and only B may claim it, because Mary-O, Sanic, the versus fighters and the
  robot lineage all still stage through the old mechanism, which is process-global);
* `PreparedCapabilityPlan`, rollback schema fragments (**C**);
* `ContentRevision` (**D**);
* any `ambition_actors` decomposition (authorised only by
  [the growth method §4](../planning/engine/api-growth-method.md)).

---

## 7. What A3 must decide that reading cannot

1. **The rollback variation** (§4). Accept `unstable_rollback_session`, or
   accept that the rollback fixture keeps a raw composition and record the fork
   in `dev/journals/code_smells.md` with `BIFURCATION:` as the first word.
2. **Where `PlatformerApp` lives.** A3 is *over current machinery, no crate
   moves*. `crates/ambition` is a facade that owns no behavior (ADR 0031:
   *"if the facade ever grows a leaf system, it has become the next monolith"*).
   A builder that sequences plugin groups is assembly, not a leaf system — but
   the line is thin enough to be worth naming out loud before it is crossed.
3. **Whether `without_gpu()` belongs on the public builder** or is test support.
   It exists for GPU-less CI, which is an engine concern, not a game's.
