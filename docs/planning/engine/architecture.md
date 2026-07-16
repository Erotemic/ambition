# Engine architecture — roles, dependency direction, and composition

This is the canonical current architecture. It describes responsibilities and
allowed composition, not a historical carve ledger. Current gaps are in
[`../status.md`](../status.md); execution order is in
[`../tracks.md`](../tracks.md); settled recon decisions are in
[`decisions-2026-07-16.md`](decisions-2026-07-16.md).

**Design oracle:** could another platformer be built by adding a provider and
content crate without editing reusable engine crates?

## 1. Repository shape

```text
crates/   reusable engine vocabulary, kernels, services, presentation, and hosts
game/     Ambition, demo providers/apps, and optional game-owned extensions
tools/    author-time generators, importers, validators, and publishing tools
docs/     current architecture, concepts, workflows, and historical evidence
```

The repository does not require one directory layout per conceptual tier. Cargo
direction and public ownership are the real boundaries. Named game content belongs
under `game/` even when it uses a public engine presentation or registration seam.

## 2. Engine faces and tiers

Imports generally flow downward. A top-tier aggregate may depend broadly, but a
lower tier does not reach upward to obtain product policy or presentation.

### Tier 0 — authored and interchange vocabulary

| Role | Current crates | Owns |
|---|---|---|
| **Authoring spine** | `ambition_entity_catalog` | authored entity/action/placement contracts and validation vocabulary |
| **Sprite/geometry metadata** | `ambition_sprite_sheet` | reusable sheet, frame, pack, animation, and measured-geometry schemas; provider art bindings are being evicted |
| **Sound format** | `ambition_sfx_bank` | serialized sound-bank format |
| **Flight-recorder format** | `ambition_gameplay_trace` | trace shapes and dump vocabulary |

Tier-0 types describe what may be authored or exchanged. They do not execute the
runtime behavior and do not contain a provider's closed named roster.

### Tier 1 — mathematical and simulation kernels

| Role | Current crates | Owns |
|---|---|---|
| **Movement kernel** | `ambition_engine_core` | geometry, casts, reference frames, motion models, body stepping, contact laws, traversal mechanics |
| **Platformer kinematic toolkit** | `ambition_platformer_primitives` | reusable ECS vocabulary around bodies, frames, lifecycle, transit, projectiles, and shared platformer state |
| **Clock model** | `ambition_time` | simulation, wall, observer, and entity proper-time domains |

These crates form one trusted mathematical foundation. They are not split merely
because geometry, frames, and movement can be named separately.

### Tier 2 — reusable domain services

| Role | Current crates | Owns |
|---|---|---|
| **Device to intent** | `ambition_input`, `ambition_touch_input` | physical input adapters and semantic control frames; pure touch folding will separate from visual controls |
| **Actor vocabulary and policy** | `ambition_characters` | actor/control types, perception, brains, action sets, boss decision policy |
| **Combat/action execution** | `ambition_combat` | move playback, hit volumes, hit resolution, targeting, combat effects |
| **Projectile kit** | `ambition_projectiles` | projectile vocabulary and reusable substrate behavior |
| **World IR** | `ambition_world` | rooms, graph, authored placements, moving-platform math, collision composition, lowering registry |
| **Authoring backend** | `ambition_ldtk_map` | LDtk import/runtime adaptation into the world IR |
| **Encounter kit** | `ambition_encounter` | participants, objectives, gates, lifecycle state, encounter facts |
| **Items and inventory** | `ambition_items`, `ambition_inventory_ui` | reusable item/equipment/inventory machinery; provider item identities are being evicted |
| **Dialogue and cutscenes** | `ambition_dialog`, `ambition_cutscene` | separate dialogue and scripted-sequence domains |
| **Stored shapes** | `ambition_persistence` | settings/save/quest persistence contracts and I/O |
| **Menu primitives** | `ambition_menu`, `ambition_settings_menu`, `ambition_ui_nav` | reusable navigation and menu rendering; Ambition's opinionated inventory host stays app-side until a second consumer proves a seam |
| **Audio/assets/effects** | `ambition_audio`, `ambition_asset_manager`, `ambition_sfx`, `ambition_vfx` | reusable runtime and registration mechanisms; provider identities belong above them |
| **Interaction** | `ambition_interaction` | generic interaction runtime vocabulary |
| **Portal exemplar** | `ambition_portal`, `ambition_portal_presentation` | the reference simulation/presentation split |
| **Workbench** | `ambition_dev_tools` | development-only inspection and tuning; generic runtime should not own its leaf systems |

**Cutscenes and encounters do not merge.** Cutscenes are scripted systems with
limited interaction. Encounters are interactive systems with limited scripting.
They may share small demonstrated primitives, not one universal sequence DSL.

### Tier 3 — simulation heart

`ambition_actors` owns the authority-woven live platformer simulation: body
assembly, control routing, perception, integration, body/contact consequences,
actor/world adapters, and publication of simulation facts.

It remains one crate because those responsibilities share runtime authority and
splitting them by size would risk rebuilding player/enemy/boss paths. This ruling
does not protect misplaced named content and does not pre-decide a boss carve
after action convergence.

### Tier 4 — observation and picture

| Role | Current crates | Owns |
|---|---|---|
| **Observation boundary** | `ambition_sim_view` | tick-tagged read models for presentation, headless agents, replay/netcode confirmation, and observer-relative views |
| **Default picture** | `ambition_render`, `ambition_portal_presentation`, `ambition_load_presentation` | sprites, camera, HUD/UI, and other presentation consumers |

Presentation consumes simulation/read-model facts and does not mutate simulation.
Immutable authored world IR may be read directly when no observer-dependent or
mutable truth is being hidden.

### Tier 5 — assembly and reusable host surfaces

| Role | Current/future owner | Owns |
|---|---|---|
| **Simulation assembly** | `ambition_runtime` | headless-safe plugin composition and global schedule-set ordering |
| **Platformer provider lifecycle** | `ambition_platformer_provider` | typed preparation, exact activation, session construction, cleanup |
| **Windowed host** | `ambition_host` | device/window/presentation composition above the runtime |
| **Programmatic harness** | `ambition_sim_harness` (landed) | reset/step, typed actions, observations, reward/termination adapters; `SandboxSim::build` takes a caller-supplied composition so it links no product shell |
| **SDK facade** | `ambition` | curated re-exports and convenient composition, not substantive lifecycle implementation |

### Tier 6 — games and providers

A game/provider owns named worlds, characters, items, art, audio, encounters,
rules, UI policy, and product presentation. Its composition root explicitly
registers the provider plugins it ships. Explicit registration is intentional;
opaque plugin discovery is not a goal.

A game-owned extension crate is appropriate only after a coherent optional piece
already depends exclusively on public engine surfaces. Do not mint extension
crates speculatively.

## 3. Domain plugin ownership

Each domain should have:

1. owned vocabulary and durable state;
2. one clear mutation authority for noncommutative state machines;
3. domain-local systems and public schedule sets;
4. typed commands/facts or narrow adapters at domain boundaries;
5. an owner plugin that initializes and installs its local implementation.

`ambition_runtime` owns the global phase graph. It orders domain sets rather than
becoming the installation site for every leaf system. Multiple writers are fine
for explicitly append-only or commutative registries; they are not a substitute
for an owner of mutable state-machine truth.

## 4. Provider and content seams

Provider-owned content enters through App/session-local catalogs,
registrations, authored fragments, or presentation plugins. A content eviction is
complete only when the engine-owned closed table disappears and the obvious
provider-owned seam exists.

The host composition root may explicitly name every installed provider. Lower
engine crates do not depend on game crates, and the `ambition` facade must not
hide product implementation behind convenient re-exports.

## 4b. Authored world IR and lowering

The world IR remains backend-neutral and uses a closed, editor-visible common
Tier-0 placement schema. Authoring backends write typed records; simulation and
content interpreters read them without making the world crate depend on runtime
actor/content types.

**Lowering** translates a typed authored placement record into its canonical live
session-scoped ECS representation. Deserialization/import has already happened.
One App-installed lowering registry is authoritative for activation, reset,
transition, and restore.

The closed common schema is settled. The only open extension question is whether
a future provider-owned placement channel exists alongside it; that question does
not justify parallel lowering authorities today.

## 5. Session authority

A prepared provider value is immutable load output. Live gameplay authority
belongs to the exact session root and its scoped entities/relationships.
Process-global mirrors of session handles or room-derived state are migration
residue unless they are explicitly scoped deterministic caches with mechanical
invalidation.

Session work has two independent acceptance gates:

1. a leak-free sequential second session/provider switch;
2. exact reset/restore reconstruction parity.

Moving a bag of raw handles onto the root is not automatically a solution. First
ask whether each handle should be derived from control/session relationships or a
read model instead.

The session-isolation gate and supported same-active-room reconstruction gate are
met (2026-07-16). The former `SceneEntities` process-global handle bag was
**removed**: the home avatar is discovered by `PrimaryPlayerOnly`, and HUD/quest
roots by their session-scoped `HudText`/`QuestPanelText` markers.

`MovingPlatformSet` is a lifecycle-scoped active-session cache under the current
one-live-session host contract. It is rebuilt by every canonical room-construction
path, registered snapshot state for exact within-room rollback, and cleared on
teardown; the resource type does not independently encode a session ID.

A provider-installed session-teardown pass resets the remaining active-session
resource mirrors when the scope retires, so no stale mirror or dangling handle
survives into the next activation. No compatibility handle bag was retained.
Atomic replacement of the live room is also landed: restore stages the snapshot's
room through canonical construction, rebuilds provider/content-authored occupants,
and then reconciles registered state. See [`netcode.md`](netcode.md) for the exact
supported boundary and remaining dynamic-family recipes.

## 6. World and geometry rules

Authored room geometry is base input. Transient runtime contributions are read
through a composited collision world. Permanent gameplay changes use an explicit
base-plus-delta model rather than silently mutating authored base geometry.

Path-dependent mechanics use swept evaluation. Reference-frame and time-domain
semantics are explicit at trusted boundaries; ordinary games use the identity
case of the same model.

## 7. Navigability and enforcement

- One module should have one clear concern, but line count alone does not create a crate boundary.
- `MODULES.md` files describe real ownership and source layout.
- Delete migration facades and duplicate execution paths when the universal path lands.
- Prefer compiler-visible dependency/visibility/type boundaries and behavioral acceptance tests.
- Add a scanner or poison fixture only for a concrete recurring harmful state that those mechanisms cannot express.
- Keep historical measurements and execution narratives in the archive or git history, not in the canonical architecture.
