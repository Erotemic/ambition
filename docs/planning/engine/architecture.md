# Engine architecture — the crate set, the roles, and the source layout

**Rewritten 2026-07-06 by fable** (supersedes the 07-04/07-05 versions; the
carve playbook that gets us here is [`decomposition.md`](decomposition.md)).
This is the canonical answer to *"what crates exist at the end state, what
does each own, which way do imports flow, and where does source live?"*

**Roles are the evergreen handles.** Every crate below carries a bracketed
ROLE — e.g. *[the sim heart]*, *[the space IR]* — and all other planning
docs (demos, doctrines, tracks) refer to crates BY ROLE. Crate names,
module paths, and file contents may drift; the roles do not. If a crate is
ever renamed or split, the role tag moves with the responsibility and this
table is updated in the same commit (living-plan discipline).

> Agent-navigability is the real goal: right abstractions, named content
> out of engine crates, and pieces small enough that a mid-tier agent can
> hold one crate's whole contract in its head.
>
> **The design oracle:** *could another platformer be built by ADDING a
> content crate without editing core?*

---

## 1. The workspace layout (the push target)

The filesystem states the oracle: engine, game, and demos are physically
separate trees. (E7 landed: `ambition_content`/`ambition_app` now live under
`game/` as drawn.)

```
crates/           — THE ENGINE. Only role-bearing engine crates. No named
                    game content anywhere under this tree (grep-enforced).
game/             — AMBITION, the first customer:
  ambition_content/   game data + content plugins + Yarn + worlds
  ambition_app/       the thin shell + binaries + full-stack tests
demos/            — the acceptance games, one directory per demo:
  demo_sanic/       sanic_content/  sanic_app/
  demo_mary_o/      mary_o_content/  mary_o_app/
  demo_smash_siblings/  ssb_content/  ssb_app/
  demo_hollow_lite/ hollow_content/  hollow_app/
tools/            — author-time tooling (sprite/music renderers, ldtk_tools)
docs/planning/    — this plan (single source of truth)
```

*(As shipped today, demos live under `game/` as `game/ambition_demo_{sanic,smb1}{,_app}`; the `demos/` layout above is the unrealized target naming.)*

## 2. The crate set (end state), by tier

Imports flow strictly downward through the tiers; within a tier only the
arrows listed here are legal (anti-god-structure rule 4).

### Tier 0 — data & format vocabulary *(no Bevy App; serde + pure fns)*

| Crate | ROLE | Owns | Must never contain |
|---|---|---|---|
| `ambition_entity_catalog` | **[the authoring spine]** | entity contracts; the `MoveSpec`/`MoveWindow`/`HitVolume` timeline schema; `EffectRef{key,params}` + param-schema checks; prefab/verb vocabulary | systems; content keys |
| `ambition_sprite_sheet` | **[the sprite-geometry authority]** | sheet/pack metadata, frame boxes, anim registry, measure-derived collision/hurtbox/attack geometry (M7: ONE pipeline), mode→sprite-state rows | rendering; game sprites |
| `ambition_sfx_bank` | **[the sound format]** | `.sfxbank` reader | — |
| `ambition_gameplay_trace` | **[the flight recorder]** | trace format + OOB dump vocabulary | — |

**Authored SCHEMA vs runtime COMPONENT (Jon+GPT-5.5 ruling 2026-07-06).**
Tier 0 owns the *authored-schema vocabulary* — the closed, editor-visible,
serde-only set of "things that can be authored/placed" (a spawn-spec, a
hazard-spec, a kinematic-path-spec, a spout-spec, a respawn-policy). This is
DISTINCT from the runtime sim COMPONENT a higher tier builds from it (the live
brain, the live hitbox). A Tier-0 schema carries NO systems and NO runtime
behavior; it is the shared contract the authoring backend WRITES and a sim/
content interpreter READS, so neither imports the other's runtime types. See
§4b (the authored-placement model) — this distinction is what keeps [the space
IR] pure while authored maps still declare rich content.

### Tier 1 — kernels *(pure simulation math; Bevy types only incidentally)*

| Crate | ROLE | Owns | Must never contain |
|---|---|---|---|
| `ambition_engine_core` | **[the movement kernel]** ([ADR 0024](../../adr/0024-frame-aware-unified-movement-kernel.md)) | the cast library (ONE home for swept AABB/circle/segment/portal-aware queries — collision doctrine CC1); one explicit swappable `MotionModel` + frame-aware `step_motion` facade over sibling axis-swept, surface-momentum, and adhesive-crawler policies; body clusters; canonical per-tick `MotionFrame` (independent reference basis + world acceleration); per-room geometry types (`World`, `Block`, `SurfaceChain`); abilities mask; tuning | rooms/authoring IR; ECS systems; content |
| `ambition_platformer_primitives` | **[the kinematic toolkit]** | kinematic stepping, the frame environment (gravity/force zones, `FrameEnv::resolve`, the per-body `ResolvedMotionFrame` artifact — ADR 0024), lifecycle, projectile primitive, transit/frame helpers, body markers | — |
| `ambition_time` | **[the clocks]** | time domains (ADR 0010/0011), `WorldTime`, proper-time policy, `ClockScaleRequest` | camera easing (lives with [the observation boundary]) |

### Tier 2 — domain service kits *(one plugin crate per domain)*

| Crate | ROLE | Owns | Key arrows |
|---|---|---|---|
| `ambition_input` / `ambition_touch_input` | **[device→intent]** | device mapping → `ControlFrame`; slot bindings (netcode N1.1) | → kernels |
| `ambition_characters` | **[the actor vocabulary]** | brains (universal `Brain::tick`, smash template, the fighter brain FB1–FB6), perception/`WorldView`, `ActionSet`, boss patterns/behavior profiles, control vocabulary | → catalog, kernels |
| `ambition_combat` | **[the combat resolver]** | hit vocabulary (`HitEvent`, volumes, hitbox lifecycle), the ONE victim-side resolver, targeting, hazards, **the moveset runtime** (playback, cancels CM4, prefab expansion), knockback/DI math (CM1/CM2) | → catalog, characters, kernels. **Never imports [the sim heart]** |
| `ambition_projectiles` | **[the projectile kit]** | projectile vocabulary/components, visual-kind art/expiry cues, spawn pools, substrate spawn executors, portal transit; victim-routing steppers stay in the sim heart until their actor/world inputs split | → kernels/effect vocabularies; sim consumes it |
| `ambition_world` | **[the space IR]** | rooms graph, `RoomSpec`/authored placement RECORDS over Tier-0 schemas (§4b), placement lowering registry, moving-platform math, `RoomMetadata` (incl. `mode`), baked `ron-room` | W3 first cut is real: zero LDtk/backend/app/render/runtime/content deps (dep-test enforced). Remaining cleanup: legacy typed family payloads on `RoomSpec` dissolve through placement lowering until the end-state names Tier-0 authored schemas only. |
| `ambition_ldtk_map` | **[the LDtk backend]** | LDtk parse/spine/entity converters, manifest/loading/hot-reload, `bevy_ecs_ldtk` runtime spine; the ONLY crate that knows LDtk exists | → world. No upward gameplay-core/app/render/runtime/content deps (dep-test enforced). A future Tiled/Godot importer is a SIBLING (Q27: deferred until truly needed) |
| `ambition_encounter` | **[the set-piece kit]** | wave/arena-lockdown specs, headless encounter state machine, encounter registries/events/music intents, reward math | → kernels, interaction, persistence |
| `ambition_items` | **[the stuff kit]** | item/inventory/equipment machinery + policies (equipment-as-armor, drop-on-hit), shop, inventory-UI state | → combat |
| `ambition_dialog` | **[the words runtime]** | Yarn runtime + lint; speaker/subject context vars | game bindings stay in content/sim |
| `ambition_persistence` | **[the saved shapes]** | save I/O, settings MODEL/schema, flags, quest progression rules | zero menu/UI imports (dep-test) |
| `ambition_menu` | **[the menu stack]** | menu model/IR, renderers, settings pages, host stack | → persistence |
| `ambition_audio` | **[the authored-audio runtime]** | Kira runtime, music intents/registries | — |
| `ambition_asset_manager` | **[the asset gate]** | catalog/profiles/loading, publish/hygiene, asset-source registration | — |
| `ambition_sfx` / `ambition_vfx` | **[the effect vocabularies]** | procedural cues; **sim-side effect messages** (`VfxMessage` etc. — the E4 inversion puts them here) | render CONSUMES these, never defines |
| `ambition_interaction`, `ambition_ui_nav`, `ambition_cutscene` | **[small kits]** | as named | — |
| `ambition_portal` / `ambition_portal_presentation` | **[the exemplar pair]** | portal sim (incl. `PortalFrame`, swept transit) / portal rendering | copy this shape for every sim/presentation split |
| `ambition_dev_tools` | **[the workbench]** | overlays, gizmos, editable tuning, profiling | may see everything; nothing sees it |

### Tier 3 — the one big crate

| Crate | ROLE |
|---|---|
| `ambition_actors` (Q2 rename pending) | **[the sim heart]** — the unified actor simulation: everything that spawns, ticks, perceives-for, and resolves the lives of BODIES. ~64k total src post-carve (the measured adapter floor, not the retired ~33k projection — see decomposition.md's LEDGER); deliberately ONE crate (splitting the actor sim would re-fork the unification); navigability is won by the internal layout below + the module standard. |

Internal module layout (the target; every module ≤ ~1.5k lines, one
concern, header stating authority + seams; `MODULES.md` at crate root):

```
src/
  spawn/       actor assembly: archetype+catalog → body (spawn_actors split up)
  tick/        integration drivers: actor update loop, home-body integration,
               MotionModel dispatch (calls [the movement kernel])
  control/     the two-port seam: SlotControls routing, ControlledSubject,
               possession, ControlGrant, brain tick bridge
  abilities/   the traversal kit: blink/dive/grapple/dash cooldowns
               (D-B carve candidate — only if re-measurement is clean)
  damage/      victim-side routing into [the combat resolver]'s facts
  mounts/      ADR 0020 link/dissolve/steer + limb routing
  perceive/    WorldView/BrainSnapshot builders (feeds [the actor vocabulary])
  policy/      respawn (ADR 0022), factions, aggression, grudges
  modes/       body modes (crouch/morph), BodyBaseSize
  session/     lifecycle, room reset, save_sync
  roster/      character catalog install seams, wear (possession semantics)
```

### Tier 4 — observation & picture

| Crate | ROLE | Owns |
|---|---|---|
| `ambition_sim_view` | **[the observation boundary]** | the `SimView` read-model (tick-tagged; per-body pos+VELOCITY; observer velocity; `PresentationFact` channel), camera snapshot + easing. Render, RL observation, netcode confirmation, and slower-light shaders are all the SAME KIND of consumer. Builders are functions of sim state. |
| `ambition_render` | **[the picture]** | sprites/camera/HUD/dialog-UI; ONE registered full-screen post-pass seam (AJ14). Depends on [the observation boundary] + Tier 0–2 vocabularies. **Never on [the sim heart]** (boundary-test enforced). |

### Tier 5 — assembly

| Crate | ROLE | Owns |
|---|---|---|
| `ambition_runtime` | **[the sim assembly]** | `PlatformerEnginePlugins` (headless-safe sim composition, owns set ordering + the schedule vocabulary — INCLUDING the per-frame player/room/portal/progression schedule wiring: headless/RL runs the same player frame a window does, E5 step-5 ruling), `add_headless_foundation`, `init_engine_states`, the mode-scope sweep (`in_mode`; the `ModeScopedEntity` marker lives a tier down in `platformer_primitives`), the `SnapshotRegistry` (netcode N3.1) |
| `ambition_host` | **[the windowed host]** | `PlatformerHostPlugins` — ONLY what a windowed game needs that headless doesn't: the leafwing input bindings/device bridge (`HostInputBindingsPlugin`) and the camera follow/shake/portal-continuity cluster (`HostCameraPlugin`). May dep [the picture]. The test "would headless/RL need this system?" decides runtime-vs-host for every future addition. |
| (post-1.0) `ambition_net` | **[the wire]** | transport trait, session shell, rollback driver | 

### Tier 6 — games (each: one content crate + one thin app)

A game/demo content crate owns: its worlds (own `.ldtk`), catalog/archetype
rows, movesets, rules plugin(s) (mode-scoped, M19), mode/match state, HUD
data. Its app owns (~100 lines): foundation plugins + [the sim assembly] +
[the windowed host] + its content plugin + host choices (window title,
asset roots, global-vs-hosted mode activation).

**Extension crates (Jon's proposal, 2026-07-06 — ADOPTED with guardrails).**
A game may also ship **extension crates**: optional, reusable plugins that
are bespoke to one game's taste but engine-clean — the exemplar is the
lunex **kaleidoscope menu backend** (Ambition's flashy menu renderer; other
games get the plain grid backend from [the menu stack] by default but MAY
pull the kaleidoscope in). Rules that keep this from overcomplicating:

1. An extension crate depends ONLY on engine crates (roles above) — never
   on its game's content crate. That is what makes it adoptable by another
   game, and it is boundary-test-enforced like every other arrow.
2. It lives in the game's tree (`game/ambition_menu_kaleidoscope/`), not
   `crates/` — the filesystem says "optional taste", not "engine".
3. **Mint on extraction, never speculatively:** a piece becomes an
   extension crate only when it is (a) optional for its own game's sim,
   (b) already engine-only in its imports, and (c) a coherent domain.
   Grow-don't-mint applies; most game code is just content.

This is the seed of a plugin ecosystem (games sharing optional backends/
modes/effects), bought without any new machinery — it is just Tier 6 with
the arrows pointed carefully. The E1e menu carve therefore splits three
ways: menu model/IR/host stack → [the menu stack] (engine); the grid
backend → [the menu stack] (the default renderer); the kaleidoscope/lunex
backend → `game/ambition_menu_kaleidoscope` (the first extension crate).

## 3. The Bevy-plugin shape (per domain crate)

1. **Owned vocabulary** — components/resources/messages native to the domain.
2. **Authoritative state** — exactly what it mutates; no other domain writes it.
3. **Local schedule sets** — `BuildIntent → Simulate → Resolve → EmitFacts →
   ProjectPresentation`; mapped into global order ONLY by [the sim assembly].
4. **Public extension slots** — content attaches to named sets
   (`CombatSet::ContentSpecials`, `ContentRoomResetSet`, …), never privates.

[The exemplar pair] is the reference implementation; copy it.

## 4. The content seams (how a game plugs in)

Install-time registries (RON + `OnceLock` resolvers, install-once): rosters,
boss profiles/sheets, item catalog, techniques, prefabs, param-schemas,
`WorldManifest`, default-character id, voice profiles (planned). Space
enters through [the space IR]'s converter registry — a content crate
registers entity converters; core ships zero worlds. Rooms may carry
`mode` for scoped game rules (M19). Classify every `OnceLock` as content
registry (seam) or immutable asset cache (no seam).

## 4b. The authored-placement model & the world→sim lowering seam

**Ruling (Jon + GPT-5.5, 2026-07-06 — this CLOSES the W3 vocab-arrow
question; future agents do NOT reopen the pure-world-IR decision.)** The
executor-facing task breakdown lives in [`decomposition.md`](decomposition.md)'s
W-track block — the implementation sub-questions [W-a..W-e] are ALL RULED
there (fable, 2026-07-06 night) with a 5-step OPUS-SAFE execution queue.

1. **The world/spatial IR stays PURE.** `ambition_world` depends on ZERO
   runtime character/combat/projectile/demo crates (not merely zero LDtk).
   Dep-test enforced.
2. **Authored maps MAY declare what spawns.** An LDtk (or any backend) file
   absolutely may say "a goblin spawns here / a falling-sand spout is here /
   this band is a hazard." The **falling-sand spout is the canonical
   example**: a spout is an AUTHORED PLACEMENT in the map, not a hardcoded
   runtime hack.
3. **Placements are AUTHORED RECORDS over Tier-0 SCHEMAS, not runtime types.**
   `RoomEmission` carries placement records whose vocabulary is the **Tier-0
   authored-schema** set (§Tier-0 note). The schema is the CLOSED,
   editor-visible vocabulary of "what can be placed"; the runtime component
   the sim builds from it lives a tier up. Jon: **prefer the closed Tier-0
   schema over a loose opaque/hybrid payload** — the author/editor should know
   the full vocabulary; a hybrid opaque RON payload is acceptable ONLY where a
   closed schema is genuinely infeasible (no strong case seen now).
4. **The world→sim LOWERING seam.** World data says WHAT EXISTS; sim/content
   systems INTERPRET it into behavior. Arrow: **sim/content → `ambition_world`,
   never the reverse.** A content/sim crate registers a lowering INTERPRETER
   keyed by schema id in [the space IR]'s converter registry (§4); lowering
   runs at ROOM-LOAD. The Tier-0 schema is the contract both the backend
   (writes) and the interpreter (reads) share without importing each other's
   runtime types.
5. **The world is NOT forever-immutable — the delta seam is RESERVED.**
   Gameplay may PERMANENTLY change the world (a destroyed wall, a dug tunnel, a
   permanently-opened gate). The architecture reserves a **base authored world
   + runtime overlay/delta** (persistable into the save as a patch) — the base
   emission/geometry is immutable input, a mutable delta layer expresses
   permanent change on top. Do NOT design lowering or SimView as if authored
   geometry is frozen for the session's life.

## 5. World-geometry rules (binding; delta seam reserved 2026-07-06)

`RoomGeometry` is authored and swapped at room boundaries. Transient
mid-room dynamics compose through the derived `CollisionWorld` overlay
(write-map enforced; only collision readers see the composited view).
**PERMANENT gameplay-driven changes** ride the reserved base+delta seam
(§4b.5) — a mutable overlay/delta on the immutable authored base, not an
in-place mutation of the authored `RoomGeometry`. (This generalizes the
transient `CollisionWorld` overlay to PERSISTED change; representation is
RULED — `WorldDelta` = ordered ops per room, save-persisted, SimView sees
only the composited view + a `WorldGeometryVersion` bump: the [W-c] ruling
in decomposition.md. Ops name geometry by **`GeoId`**, the durable
geometry-identity model — placement/tile-layer/generator/delta sources,
faces via `GeoFaceRef` — ruled in collision-and-ccd.md §3.6; the CC6
portal host ref uses the same vocabulary.) Authoring backends
own SPACE; parameterized generator entities (`SurfaceLoop`, planned
`SurfaceRamp` quarter-circle floor↔wall transitions — Q27 ruling) keep
LDtk sufficient for non-axis-aligned content without a new backend.

## 6. Validation & discipline

The differential headless harness + C4 rigs + boundary tests gate
structural work; feel ships BLIND. CI runs the workspace suite. And the
standing rule: **delete, don't bridge; rename in place; add seams when the
second use case lands.**
