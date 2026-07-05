# Engine architecture — the target crate stack

**Rewritten 2026-07-04 by fable; amended 2026-07-05** (host tier added; the
carve playbook now lives in [`decomposition.md`](decomposition.md), which is
the EXECUTION companion to this target-state doc). This is the canonical
answer to *"what crates should exist, what does each own, and which way do
imports flow?"* The actor model is [`unified-actors.md`](unified-actors.md);
the ordered teardown recipes are [`decomposition.md`](decomposition.md); the
live queue is [`../tracks.md`](../tracks.md); the phase roadmap is
[`../roadmap.md`](../roadmap.md).

> Agent-navigability is the real goal. The point is the right abstractions +
> getting NAMED content (bosses, abilities, rooms) out of engine crates into
> content, generalized where possible.
>
> **The design oracle:** *could another platformer be built by ADDING a content
> crate without editing core?* If a change makes the answer "no", it's the
> wrong change.

---

## Sizing philosophy (Jon's constraints, binding)

- **Medium, well-scoped crates.** A crate earns existence by owning a coherent
  domain with real meat — not by being small. A ~100-line vanity crate is a
  smell; so is a 100k monolith. The healthy band is roughly 1–15k LOC.
- **Grow existing crates before minting new ones.** When a module reaches its
  leaf home, prefer folding it into the sibling that already owns the domain
  (`character_sprites` → `ambition_sprite_sheet`, `time/` → `ambition_time`)
  over a new crate with one tenant.
- **Don't force splits that fight the grain.** `features/ecs ↔ combat` are
  mutually-importing *by construction* (the actor sim and its combat kit).
  The mechanics core extracts as ONE crate when its support ring is gone —
  never as two crates with a synthetic boundary between them.
- **Compile time is a first-class constraint** (ADR 0013). The decomposition
  exists to shrink the hot-edit rebuild set: hot mechanics in one crate,
  cold support (world/persistence/menu/audio/assets) in leaves, presentation
  cut loose via the read-model.

## The target stack

Imports flow strictly downward. Names are the CANONICAL targets — short names,
no `_runtime` suffix scheme (this supersedes the old
`ambition_actor_runtime`/`ambition_combat_runtime`/… lineup, which predated the
keystone and never matched the tree).

### Tier 0 — data & format leaves (no Bevy app, minimal deps)

| Crate | Owns | Delta from today |
|---|---|---|
| `ambition_entity_catalog` | Entity contracts + the `MoveSpec` timeline schema — the authoring spine | grows the JD1 ability schema: `EffectRef { key, params }`, prefab refs, verb-selection map, `on_hit` effects |
| `ambition_sprite_sheet` | THE sprite-metadata pipeline: sheet/pack data, frame boxes, anim registry, metadata→hitbox derivation | absorbs `gameplay_core::character_sprites` + `boss_encounter::{sprites, attack_geometry}` (one pipeline for collision/hurtbox/attack — M7) |
| `ambition_sfx_bank` | `.sfxbank` format reader | as-is |
| `ambition_gameplay_trace` | flight-recorder format | as-is |

### Tier 1 — foundations

| Crate | Owns | Delta |
|---|---|---|
| `ambition_engine_core` | movement/collision/blink/ledge kernel, body clusters, `AccelerationFrame`, `World`/`RoomGeometry`, config | as-is (healthy; 13.7k of substance) |
| `ambition_platformer_primitives` | kinematic stepping, gravity field, lifecycle, projectile primitive, markers | as-is |
| `ambition_time` | clock vocabulary + time domains (ADR 0010/0011) | absorbs `gameplay_core::time` (time-control authority, proper-time policy); `camera_ease` moves with the camera read-model instead |

### Tier 2 — engine service kits (one plugin crate per domain)

| Crate | Owns | Delta |
|---|---|---|
| `ambition_input` / `ambition_touch_input` | device → `ControlFrame` | as-is; touch_input's upward deps (gameplay_core/render menu-bridge) invert later |
| `ambition_characters` | actor BEHAVIOR vocabulary: brains, perception, action sets, boss patterns, body components | absorbs `boss_encounter::{behavior, registry}` (bosses are actors) |
| `ambition_combat` | **the finished extraction**: hitbox lifecycle, `resolve_body_hit`, targeting, hazards, **the moveset runtime**, chests/breakables/pickups kit | absorbs `gameplay_core::combat` (~10k) once the `features` back-edge is cut |
| `ambition_projectiles` | the projectile faction pair (player + enemy pools, unified stepping) | new home for `projectile/` + `enemy_projectile/` |
| `ambition_world` | rooms graph, LDtk runtime + **content-registered converter registry** (ADR 0009), moving platforms, physics adapter, gravity zones, `WorldManifest` install seam | new crate (D4); the JD4 seam is its keystone |
| `ambition_encounter` | wave/arena-lockdown kit + scripted encounter beats | absorbs `boss_encounter::{encounter_script, rewards}` |
| `ambition_items` | item/inventory/equipment machinery, shop, inventory-UI state | `items/` + `inventory_ui/`; the catalog DATA is content (C1, done) |
| `ambition_dialog` | Yarn runtime + lint machinery | `dialog/runtime` out of gameplay_core; game bindings stay sim-side |
| `ambition_persistence` | save I/O + settings schema + host/display vocabulary (+ quest progression rules) | `persistence/` + `host/` + `quest/` |
| `ambition_menu` | menu model + renderers + **settings IR + the menu host stack** | absorbs `gameplay_core::menu` (3.2k) AND `ambition_app::menu` (10k — the misplaced elephant); deps `ambition_persistence` |
| `ambition_audio` | authored-audio runtime (Kira), music intents | absorbs `gameplay_core::{audio, music}` |
| `ambition_asset_manager` | asset catalog/profiles/loading + publish/hygiene tooling | absorbs `gameplay_core::{assets, asset_publish}` |
| `ambition_sfx`, `ambition_vfx`, `ambition_ui_nav`, `ambition_interaction`, `ambition_cutscene` | as-is (thin but load-bearing vocabulary/kit leaves) | — |
| `ambition_portal` / `ambition_portal_presentation` | **the exemplar pair** — copy this shape for every extraction | as-is |
| `ambition_dev_tools` | debug overlays, gizmos, editable tuning, profiling | `gameplay_core::dev` + `ambition_app::dev` |

### Tier 3 — the actor simulation core (the heart, ONE crate)

**`ambition_actors`** (the renamed residue of `ambition_gameplay_core` — rename
LAST, it's mechanical): actor spawn/tick/perception/damage-routing, the player
systems, the ability kit (blink/dive/grapple/possession/ranged), body modes,
session lifecycle, schedule vocabulary, view-index builders, dialog bindings.
Estimated ~30–35k after the tier-2 evictions — a large-medium crate that is
genuinely ONE concern: the unified actor simulation. Per roadmap U1, re-measure
before any further split; do not pre-commit to one.

### Tier 4 — read-model & presentation

| Crate | Owns |
|---|---|
| `ambition_sim_view` | the MATERIALIZED read-model: `FeatureView`(+index), actor render/anim indices, boss render index, `CameraSnapshot2d` + camera-ease, sim→presentation messages. Created only when materialization is complete enough to cut the render edge (the E24 condition). |
| `ambition_render` | sprites/camera/HUD/dialog-UI — deps `ambition_sim_view` + foundations, **NOT `ambition_actors`** (the D3.7 lever) |
| `ambition_portal_presentation` | as-is |

### Tier 5 — assembly

**`ambition_runtime`** (the C4/M12 deliverable — EXISTS since `3c70d827`):
`PlatformerEnginePlugins` — a Bevy plugin group owning the SIM subsystem
ordering, headless-safe by construction (dep budget: gameplay_core +
characters + bevy). The `App::new().add_plugins(...)` moment; the single
most Unity/Godot-shaped artifact. Also owns `add_headless_foundation` (the
shared headless/RL bootstrap).

**`ambition_host`** (new, 2026-07-05 — the presentation-tier sibling):
`PlatformerHostPlugins` — the engine-generic WINDOWED host wiring (input
plugins/routing, portal schedule placement, room-transition registration,
camera policies) that may depend on `ambition_render`/`ambition_input`. A
demo app = foundation + `PlatformerEnginePlugins` + `PlatformerHostPlugins`
+ its content crate. Carve recipe: [`decomposition.md`](decomposition.md)
E5-finish. Post-1.0, the netcode ladder adds `ambition_net` here (transport
+ session; [`netcode.md`](netcode.md)) — seams only until then.

### Tier 6 — game

- `ambition_content` — Ambition's named world: rosters, worlds (`.ldtk`
  payloads via the `WorldManifest` seam), items/boss data RON, quests,
  dialogue `.yarn`, music/sfx registries + baked sprite data (the whole
  asset payload that lives in gameplay_core today), techniques,
  **falling-sand as a self-gating content plugin**, duel-arena staging.
- `ambition_app` — the thin shell: binaries, host glue, RL sim (~6–8k after
  the menu/dev evictions).
- `demos/…` — the acceptance suite ([`../demos/`](../demos/README.md)):
  Sanic, Super Mary-O, Super Smash Siblings, Hollow Lite — each ONE content
  crate + a ~100-line app; every needed core edit files an oracle-violation
  in [`../tracks.md`](../tracks.md). `ambition_app` additionally depends on
  the demo CONTENT crates to host them in-world (the mode-scope pattern,
  decomposition Phase D-C).

## Bevy-plugin shape

Each domain crate is a plugin exposing four things:

1. **Owned vocabulary** — components / resources / messages native to the domain.
2. **Authoritative state** — exactly what it mutates (no other domain writes it).
3. **Local schedule sets** — a consistent rhythm: `BuildIntent → Simulate →
   Resolve → EmitFacts → ProjectPresentation`. A domain creates a *local* set
   when its ordering is internal; it maps into a global set only in
   `ambition_runtime`.
4. **Public extension points** — content attaches systems to named slots
   (`CombatSet::ContentSpecials`, `PortalSet`, …) without reaching into privates.

**`ambition_portal` is the exemplar** — runtime core crate + optional
presentation crate + a *visible* content adapter + the runtime maps the
schedule. Its shape is why it extracted clean; copy it.

## The content seams (how a game plugs in)

All content enters through **install-time registries** (RON + `OnceLock`
pure-function resolvers — install once at startup, immutable after, readable
from non-system code). Proven instances: enemy/boss rosters, boss
profiles/sheets/strike-geometry, item catalog, techniques
(`register_required_components` off `Effect{key}`). The remaining seams to
build, spec'd in the 2026-07-04 review:

- **`WorldManifest`** — content declares its LDtk worlds + entry room;
  `ambition_world` loads them through content-registered entity converters
  (ADR 0009). Core ships zero worlds.
- **The JD1 ability model** — three tiers: authored `MoveSpec` DATA →
  parameterized PREFABS (`Prefab { key, params }` → registered constructor) →
  arbitrary-code TECHNIQUES (`Effect { key, params }` → content Bevy system).
  Params are an opaque serde value each effect hydrates into its own type;
  input→move mapping lives in the published character data (`verbs` map with
  directional resolution). Core never matches a content key.
- **Room mechanics split by kind** (JD4, adjudicated): authored `Authored<T>`
  data where it's entities; a self-gating content plugin for a heavy sim; a
  `RoomLoaded` message for imperative staging.

Classify each `OnceLock` as either a **content registry** (install seam — a
second game installs its own) or an **immutable asset cache** (derived from
baked tables, no seam needed).

## World geometry

`RoomGeometry` is **authored** and swapped at room boundaries — never mutated
mid-room. All mid-room dynamics (moving platforms, gates, portal carves, ECS
solids) compose through the **derived `CollisionWorld` view**.

**Write-map (who may mutate `RoomGeometry`):** boundary swaps only
(session/reset, world_flow, dev_runtime). Mid-room mutators must move to the
overlay — remaining: `content/bosses/gnu_ton` subtractive carve (needs the
overlay extended with carve + climbable-region), `falling_sand` (rides its
content-plugin move).

**Collision-view guard:** route only the **collision** readers through the
composited view — never render/layout/metadata readers or projectiles
(`carves_only`). A reader that suddenly sees platform/carve geometry is a
silent feel regression.

## Validation strategy

Structural refactors land **ahead of** feel/visual verification; the gate is
the **differential headless harness**
([`headless-verification.md`](headless-verification.md)) + the C4 gravity
symmetry rigs + the boundary tests. Feel-touching changes ship BLIND in marked
commits for Jon's in-game pass. CI runs `cargo test --workspace` (leaf-crate
tests rot under `-p` runs — proven twice).

## The discipline

> **Delete, don't bridge. Rename in place, don't alias. Add seams when the
> second use case lands.** Pre-release, single-commit replacement beats a
> two-step bridge. Move a type family to its real leaf home once, then
> redirect every consumer — never chase a middle facade (the D2 template).
