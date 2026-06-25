# Restructuring blueprint — actionable distillation

*Author: Claude Opus 4.8 (1M) · 2026-06-25 · status: PROPOSAL (one decision resolved; see RoomGeometry)*

This distils an externally-generated "restructuring blueprint v5" (static
inspection only, no `cargo`) into a repo-canonical plan. It keeps two kinds of
value from the source: **sequencing** (what to do first) and **orientation**
(the target shape, the domain contract, the portal extraction template, per-domain
current→target maps, search entry points) — a fresh agent needs the second even
when not acting on the first. It is filtered through standing constraints:

- **No backwards-compat tax.** Nothing depends on this repo, there is no release.
  Prefer single-commit replacement over bridge/alias/compat ceremony.
- **Narrow types over wide generic surfaces.** Add a seam (message, trait, knob)
  when a second use case lands — not preemptively.
- **Relativity principle.** No player-centrism; mechanics frame-agnostic and
  shared by every actor.
- **Elegance over hacks.** Generalise the elegant pattern already in the code;
  delete the leak. Correctness is emergent from the right shape.
- **Agent-navigability is the real goal.** ~150k LOC is too hard to navigate; the
  point is right abstractions + getting NAMED content (bosses, spells, rooms) out
  of foundation crates into content, generalised where possible.

Counts and smells below were re-verified against live `main` on 2026-06-25.

> Editorial note: where this doc demotes or rejects a v5 idea (the mass `Player*`
> rename as a wave, the message-vocab-for-everything, the bridge/alias ceremony),
> that is a deliberate filter, not an omission. The *ideas* are recorded; the
> *timing/mechanism* is changed.

---

## What's worth keeping (guardrails — do not regress these)

The repo is already substantially Bevy-ECS-shaped. The reshape is ownership
clarity, not a conversion. Preserve and amplify:

1. **ECS components for per-entity state** (body/combat/safety/input already on
   entities). Rename and re-scope, don't replace the model.
2. **Resources for true globals** — caches, registries, settings, asset handles,
   explicit game-mode state.
3. **Messages for frame facts/requests** where ownership is clear.
4. **System sets as semantic frame phases** (the `SandboxSet` spine is valuable).
5. **Plugins as installable domains** (portal, time, asset-manager, mobile input,
   content install already point the right way).
6. **Architecture boundary tests** — keep as guardrails; add canonical-import and
   concept-leak checks rather than discard.
7. **The headless simulation path** — the key validation target for
   sim/presentation separation and future server simulation.
8. **The portal split** — the best current exemplar of runtime core + presentation
   + content adapter + host schedule mapping (see Target architecture).

---

## Resolved decision: `GameWorld` → `RoomGeometry`, read through a collision view

The blueprint posed an open fork: is `GameWorld` an *authoritative mutable world*
or a *derived cache*? That fork was a false dichotomy built on a bad name. There
is no cache anywhere.

**What the type actually is.** `ae::World` is
`{ name, size, spawn, blocks, water_regions, climbable_regions }` — purely the
**static spatial geometry of one room**: bounds, spawn, collision blocks, water,
ladders. No entities, no actors, no items, no dynamic state. `GameWorld` is the
Bevy-resource wrapper around it. The `Game` prefix carries no meaning; it exists
only to avoid clashing with `bevy::ecs::World`. The name is named for what it
*isn't*.

**How it behaves today (already a clean split, just unnamed):**

- `GameWorld` is **authored, write-once-per-room.** Every production write is
  wholesale replacement at a room boundary — `world.0 = spec.world.clone()` in
  `room_flow.rs`, `session/reset/mod.rs`, `dev_runtime.rs`, plus the initial
  insert. Nothing in production mutates it incrementally mid-room. (The
  `gnu_ton.rs` `.0 = World::new(...)` writes are test scaffolding simulating room
  changes, not a content hack.)
- The **mid-room dynamics are a derived view, not mutation.** Moving platforms,
  ECS solids, and portal carves fold into a *fresh* `ae::World` each frame via
  `combat::world_overlay::world_with_sandbox_solids`, with a `Cow::Borrowed` fast
  path so the no-dynamics case never clones. Portal core owns carve geometry and
  is forbidden from naming the host overlay (`FeatureEcsWorldOverlay`); Ambition
  owns how a carve alters collision.

**Decision:**

1. Rename the resource `GameWorld` → **`RoomGeometry`** (authored, swapped at
   room/reset/hot-reload boundaries). The engine `ae::World` may keep `World`
   (physics-engine idiomatic; `ae::` disambiguates) or later become `Terrain` —
   lower priority than the resource wrapper.
2. The per-frame composite is a **collision view**, not a cache: a computed
   `RoomGeometry + overlay` value, transient. `FeatureEcsWorldOverlay` is the
   retained per-frame *gather* of dynamic contributions (platforms, ECS solids,
   carves) — the overlay layer the view composites over.
3. **The 25 raw `Res<GameWorld>` readers are the bug.** They read bare geometry
   when they should read the collision view. Promote the composite to the single
   collision read-API and route readers through it. This is the *same seam* the
   collision-semantics dedup needs (plan item 2) — one frontier.

Why this is the elegant answer and not the mutable pole: an authored
`RoomGeometry` + derived collision view is replay/RL-friendly (a frame's collision
truth is a pure function of room id + overlay state — snapshot/rewind is free) and
naturally supports per-player world variants later, without a mutable monolith
that tempts content to reach in and mutate the base.

---

## Target architecture (reference — the north star shape)

Not all of this gets built now. It is where code is heading so cleanup patches
share a direction.

### Runtime vocabulary (the role taxonomy)

The word `player` compresses six distinct roles. The taxonomy is a thinking tool
*now* even though the renames land gradually (see "Do opportunistically"):

```text
Actor             simulated entity with body/combat/inventory/brain/lifecycle.
ControlledActor   an actor receiving intent from an authority this frame.
InputSource       keyboard/gamepad/touch/AI/script/replay/remote raw|normalized input.
Participant       local/remote human/session endpoint, spectator, replay viewer, debug harness.
ControlAuthority  policy mapping input sources or brains to actor intent.
ActorIntent       entity-local simulation input consumed by body/item/combat systems.
Viewpoint         camera/render observation policy (usually follows a controlled actor).
PresentationFocus participant-scoped target for HUD/UI/aim hints/local-only visuals.
```

`player` stays valid for human-facing UI copy and temporary migration labels; it
stops being the *core simulation concept*.

### Target crate / plugin families

A shared game-runtime phase spine with vertical domain families (not one giant
horizontal split):

```text
ambition_game_runtime      phase vocabulary, lifecycle windows, game mode/run conditions,
                           save/reset/room contracts
ambition_actor_control     input-source snapshots, participant/authority routing, ActorIntent
ambition_actor_runtime     body/control/brain integration for actors that move and act
ambition_world_runtime     room/LDtk/load/reset/lifecycle/RoomGeometry authority
ambition_carryable_items   item identity, holder relationship, world/resting/thrown states,
                           pickup/drop/use transitions
ambition_projectiles       projectile body/lifecycle/messages with source/cause attribution
ambition_combat_runtime    hitboxes, damage, attribution, factions/teams, combat facts
ambition_encounter_runtime encounter scripts, boss phase/runtime, payload release, wave registries
ambition_cutscene_runtime  cutscene playback state, queues, advance requests, save/progression effects
ambition_<domain>_presentation   render/view facts and visuals for a domain when substantial
ambition_content           authored Ambition rows + installation; subcrate-split only when useful
ambition_game              canonical composition root: installs content, maps domain sets into phases
ambition_app               executable/platform host: windows, devices, lifecycle, asset sources, binary opts
```

### What makes a real domain plugin (the contract)

A genuine domain plugin is an ownership package, not an `add_systems` dump. It
makes five things obvious:

```text
Domain vocabulary       components/resources/messages/types native to this domain.
Authoritative state     what the domain owns and mutates.
Fact/request/event vocab facts (something happened), requests (please consider this),
                        message transport for both.
Local schedule sets     domain-local sets (BuildIntent, Simulate, Resolve, EmitFacts,
                        ProjectPresentation); the composition root maps them into the spine.
Host-facing extension    public sets/resources/messages where content/adapters/presentation
points                  attach without reaching into private internals.
```

> Filter: define the fact/request/event *messages* when a **second** consumer
> lands. A `StartCutsceneRequest`/`CutsceneStarted` pair for a one-producer,
> one-consumer domain is premature indirection (and we've been bitten by
> query-order determinism). The *contract shape* — owned state, local sets,
> extension points — is not premature and is the bar a "real" plugin must clear.

### App composition contract (app owns composition, not semantics)

`ambition_app` should answer: *which plugins are installed for this binary; which
content pack; which platform/device/window backends; which domain-local sets map
into which global phases; which dev plugins for this profile.* It should **not**
define what a domain transition *means*. Current app files still hosting domain
semantics that should drain into plugins:

| app location | semantics hosted there | target owner |
| --- | --- | --- |
| `app/sim_systems.rs` | input sync, brain tick, room transition, reset/replay, interact glue | control/actor runtime, world runtime, effect/interact adapters |
| `app/combat_schedule.rs` | actor actions, boss specials, effects, projectile stepping, hitbox/damage order | combat/projectile runtime + content extension sets |
| `app/progression_schedule.rs` | room-entry facts, checkpoint/shrine/dialogue, room music, portal tick | progression facts, world runtime, content adapters, audio, portal |
| `app/plugins.rs` | broad sandbox sim/presentation/LDtk composition | `ambition_game` root with app as host shell |
| `app/sim_resources.rs` | bundle of resources/messages for many domains | owning domain plugins register their own |

### Portal as the extraction exemplar (copy this shape)

The portal family is the concrete template every future domain extraction should
imitate:

```text
ambition_portal               portal runtime vocabulary, resources, messages, PortalSet
ambition_portal_presentation  visual projection / presentation systems
ambition_content/src/portal   Ambition adapters: input, movement intent, room reset, items, SFX,
                              world transition — the glue, visible AS glue
ambition_app / ambition_game  maps portal runtime/presentation/adapters into the schedule
```

Why it is the exemplar: (1) the reusable mechanic is not buried in
gameplay_core; (2) presentation is separate from runtime; (3) Ambition-specific
glue is a visible adapter, not pretending to be generic; (4) it exposes a local
set (`PortalSet`) rather than forcing callers to know the whole sandbox schedule;
(5) its remaining impurities are concrete adapter responsibilities (still uses
`ControlFrame`, gameplay-core shims) — migration work, not reasons to recollapse.
**Template for any extraction: runtime core, optional presentation, optional
content/adapter package, host schedule mapping.**

---

## The plan, ordered by value

### 1. Delete the compatibility shims (one canonical import per concept)

`ambition_gameplay_core/src/lib.rs` re-exports already-extracted crates under
historical paths, creating multiple valid import paths for one concept — directly
against agent-navigability. Live call-site pressure (excluding gameplay_core):

| shim | canonical | live hits |
| --- | --- | --- |
| `::kinematic` | `ambition_platformer_primitives::kinematic` | **0** |
| `::ui_nav` | `ambition_ui_nav` | 3 |
| `::interaction` | `ambition_interaction` | 6 |
| `::actor` | `ambition_characters::actor` | 16 |
| `::brain` | `ambition_characters::brain` | 37 |
| `::engine_core` | `ambition_engine_core` | 68 |
| `::input` | `ambition_input` | 70 |

**Do:** delete each shim, fix imports, one commit per shim — **no facade, no
allowlist, no deprecation window** (no external consumers). Start with `kinematic`
(free) and `ui_nav`/`interaction`. Add an architecture-boundary test that fails on
new internal use of these paths — keep the *test*, not an alias. Second batch
(`actor`/`brain`/`interaction`) crosses content/render/boss code, so do it after
those crates' imports are ready.

**Validation:** `rg "ambition_gameplay_core::(input|engine_core|brain|actor|interaction|ui_nav|kinematic)" crates` → zero internal hits.

### 2. Collision/support-semantics dedup (+ RoomGeometry collision view)

The highest-value correctness work. Two implementations carry overlapping
gravity-relative support semantics that can agree at the design level while
drifting at the implementation level:

- `ambition_engine_core/src/movement/collision.rs` (707 lines) — controlled-body.
- `ambition_platformer_primitives/src/kinematic.rs` (1226 lines) — generic
  actor/NPC/enemy sweep.

This is the relativity principle as a correctness property: every actor — player,
NPC, enemy, projectile, remote/AI — collides against one composited truth. It is
the engine-for-other-games keystone.

**Do (parity-first — the proven-safe order for big mechanical ports):**

1. Shared fixture table: `BlockKind` × cardinal `gravity_dir` × previous-feet
   coord × delta × drop-through → expected support/block/pass.
2. Run identical expectations against *both* paths before changing anything.
3. Extract pure helpers (support-surface classification, gravity-axis role,
   support-face separation, one-way landing eligibility) into a shared semantics
   module; keep both sweeps but make them call it.
4. Land the `RoomGeometry` collision-view API here — both sweeps query the
   composited view, not bare geometry. Unifies item 1's decision with the dedup.
5. After parity holds, decide if controlled-body movement consumes
   `step_kinematic` directly or keeps a richer sweep over the same kernel.

Watch the dependency direction: `platformer_primitives` already depends on
`engine_core` for `Aabb`/`Block`/`BlockKind`/`World`, so the shared semantics home
needs a clean direction.

### 3. Drain simulation out of `ambition_app` into domain plugins

The app should compose and host, not define domain meaning (see app contract).
Clearest first movers (low coupling), from `app/sim_systems.rs`:

- `attack_advance_system` → combat runtime.
- `detect_room_transition_system` → world runtime (after the RoomGeometry
  write-map exists).
- `apply_player_hit_events` → combat/actor-health runtime (+ source/cause).

Keep platform/device/Android/mobile/window systems in app. `ambition_game` is a
*direction*, not a prerequisite — introduce it when the app file reads as two jobs
(host vs. compose). **Preserve ordering-sensitive comments AS tests** when moving
systems — projectile-spawn timing especially.

### 4. `ControlFrame` → actor-local intent

`ControlFrame` is a fine input-source snapshot; the problem is ~46 systems read
the global `Res<ControlFrame>` directly, hardcoding one local input source and one
primary controlled actor — the player-centrism the relativity principle rejects.
Keep `ControlFrame` as input-source data; move *simulation* onto entity-local
`ActorIntent`/`ActorInputFrame`. Treat render/mobile joystick readers as
presentation consumers, not simulation authority.

**First converts (one at a time, behaviour-preserving):**

1. `heal_save_shrine_system` → actor-local interact/use intent (smallest).
2. `compute_player_intent` → `compute_controlled_actor_intent`; centralise ability
   use decisions there instead of each ability re-reading global input.
3. One ranged ability (`fire_shockwave_system`) as the pattern.
4. Carryable-item use/throw/fire (`throw_held_item_system`, `fire_held_ranged_system`)
   onto actor/item intent + holder relationship.
5. Portal input adapter last (after core consumers move).

**Validation:** remaining direct `Res<ControlFrame>` uses cluster in input-source
*writers*, tests, and presentation — not ability/item/combat sim.

### 5. Classify the `OnceLock` global registries

Eight `OnceLock`s (boss profiles/specs, enemy roster, encounter waves, sheet
indices). Not automatically wrong. **Classify each** as content registry,
immutable asset-metadata cache, or test-override seam. Promote content registries
(`ENEMY_ROSTER_OVERRIDE`, `BOSS_PROFILE_OVERRIDE`, `BOSS_ENCOUNTER_SPEC_OVERRIDE`,
`ENCOUNTER_WAVE_BOOK`) toward resources/contexts; keep pure immutable sheet/index
caches but *name and document them as asset caches*. Low urgency relative to 1–4.

---

## Domain-by-domain: current → target → first move

Condensed orientation so an agent can work a domain without rediscovering it.

### Input / control / controlled actors
- **Now:** `ControlFrame` global → `sync_local_player_input_frame` mirrors it onto
  the primary body as `PlayerInputFrame` → brain/action systems emit `ActorControl`
  /`ActorActionMessage`; many abilities still read the global directly.
- **Target:** InputSource frames → ControlAuthority routes → entity-local
  `ActorIntent`; sim consumes intent, presentation consumes Viewpoint/focus.
- **First move:** plan item 4.

### Actor / body / brain runtime
- **Now:** `engine_core` owns body/control types with `Player*` names;
  `ambition_characters` owns actor/brain + a hardcoded held-item/action-set
  registry; `gameplay_core::player` owns the controlled-character ECS; app
  schedules the sim.
- **Target:** actor runtime owns body/control/brain ECS and advances all
  controlled/scripted/AI actors through the same systems; named item/action rows
  live in content.
- **First move:** role-audit `player_clusters.rs` + `player/components/mod.rs`;
  move named held-item/action rows out of `characters` into content install rows;
  move app-owned actor-sim registration toward an actor-runtime plugin.

### Carryable items
- **Now:** held and thrown are states of one lifecycle; `HeldItemSpec` flows
  held↔world; `GroundItem` is resting/thrown; `ItemPickupSimulationPlugin` owns
  pickup/throw/free-body/thrown-effects/wielded-abilities.
- **Target:** `CarryableItemRuntimePlugin` with `ItemInstanceId`, `ItemSpecId`,
  holder relationship, world physical state, pickup/drop/throw/recover transitions,
  actor-intent use dispatch, source/cause attribution; item-effect extension
  plugins (bomb, gravity grenade, slug gun, ranged) ; item content install.
- **First move:** keep held/thrown unified; add the lifecycle state enum/map;
  convert use/throw/pickup to actor-local intent; add instance-identity + holder +
  source fields **before** any multiplayer (compact, needed even single-player for
  attribution/save).

### Projectiles / combat
- **Now:** `ambition_combat` has primitives; **`platformer_primitives` carries
  named spell vocabulary (`Fireball`, `Hadouken`) — named content in an engine
  crate (verified)**; gameplay_core `projectile`/`enemy_projectile` split by
  player/enemy; `CombatSchedulePlugin` mixes actor actions, ~11 boss-special
  consumers, effects, projectile stepping, hitboxes, content flavor.
- **Target:** `ProjectileRuntimePlugin` (generic body/lifecycle/messages + source
  actor/item/faction/authority-tick) + `CombatRuntimePlugin` (hitbox/damage/facts/
  attribution) + content plugins for named projectile kinds and a
  `BossSpecialContentPlugin` mounting specials into an explicit `CombatSet::
  ContentSpecials` extension set; `EncounterFlavorContentPlugin` for cut-rope etc.
- **First move:** add source/cause attribution to projectile/damage messages;
  pull boss-special consumers into the content plugin + extension set; split
  generic stepping from named kinds; replace player/enemy split with source/faction.
  Keep the projectile-spawn timing contracts (encoded in comments → make tests).

### World / rooms / LDtk
- **Now:** `RoomGeometry` (was `GameWorld`) is the per-room geometry; `world/
  ldtk_world` owns conversion/runtime; app detects/applies room transitions;
  some content mutates world for portals/boss gates/intro.
- **Target:** `WorldRuntimePlugin`/`LdtkWorldPlugin` owns load/reset/lifecycle +
  the RoomGeometry contract + the collision-view read API; content mutates world
  through explicit commands/facts, not ad hoc access.
- **First move:** a RoomGeometry write-map (every writer + why), classify each
  (LDtk load / transition / dynamic feature / portal carve / content gate / test),
  define world commands/events before moving code.

### Content / adapters
- **Now:** `ambition_content` mixes authored rows, install plugins, and runtime
  adapters; **it imports `ambition_render::cutscene` runtime resources (verified
  boundary leak)**; portal adapters still use global `ControlFrame` + gameplay-core
  shims.
- **Target:** module families `content::{authored, install, adapters,
  presentation_bindings}`; adapters are explicit domain→domain translators mounted
  into extension sets.
- **First move:** split the module families (cheap conceptual reorg, not crate
  split); move portal/boss adapters into explicit adapter modules; treat quest as
  facts/commands only.

### Cutscenes / dialogue / render — a clean bounded win
- **Now:** `ambition_cutscene` exists but **cutscene runtime types
  (`CutsceneLibrary`, `ActiveCutscene`, `CutsceneTriggerQueue`,
  `CutsceneAdvanceRequest`, `RoomCutsceneBindings`, `CutsceneSchedulePlugin`) live
  in `ambition_render::cutscene` (verified)**; content inserts those render
  resources; docs already call this boundary debt.
- **Target:** `ambition_cutscene` owns runtime (queues/playback/advance/room
  bindings/save effects); `render::cutscene_presentation` owns UI/render; content
  installs scripts/bindings into the *runtime*, not render.
- **First move:** classify every type/system in `render::cutscene` as runtime /
  presentation / authored-default / save-effect; move runtime-only vocabulary to
  `ambition_cutscene`. Self-contained, high clarity-per-effort.

### Audio / music / SFX / VFX
- **Now:** `SandboxAudioPlugin` mixes backend, settings, content cue loading, and
  playback (verified — inits radio/sfx-bank/environment/default-music together).
- **Target:** `AudioRuntimePlugin` (backend-independent playback) +
  `MusicDirectorPlugin` (room/combat/cutscene intent → cue) + `Sfx/VfxRuntimePlugin`
  (explicit messages + source/cause); content owns named cue/bank mapping.
- **First move:** map all `SfxMessage`/`VfxMessage` producers/consumers; separate
  cue mapping from playback backend; add source/cause where missing.

### Progression / quest
- **Now:** quest is underdeveloped scaffolding; `progression_schedule.rs` mixes
  boss runtime, quest pumping, room metadata, map visits, save sync, dev inspector.
- **Target:** `ProgressionFactsPlugin` (durable facts from world/combat/cutscene/
  item) + `TemporaryQuestScaffoldPlugin` (current rewards/flags, replaceable) +
  future real quest runtime.
- **First move:** isolate quest-facing systems behind fact/command messages so the
  current quest code is easy to replace; preserve facts + save boundaries; do not
  design around today's quest implementation.

---

## Plugin promotion candidates (recommended order)

1. **Control/actor-intent plugin** — absorb the `ControlFrame` bridge, brain tick,
   action emission, future authority routing.
2. **Carryable-item lifecycle plugin** — unified held/world/thrown/recovered +
   instance identity + holder + attribution.
3. **Combat runtime plugin** — hitboxes, damage, facts, attribution, faction rules.
4. **Projectile runtime plugin** — generic body lifecycle + spawn/despawn messages;
   split from named spells/item abilities.
5. **Cutscene runtime plugin** — move runtime out of render (the bounded win).
6. **World/LDtk runtime plugin** — room load/reset/lifecycle + RoomGeometry contract.
7. **Encounter runtime plugin** — scripts/phases/payloads; content specials mount
   into extension sets.
8. **Audio/music runtime plugin** — split backend / settings / cue / director.

Portal remains the *exemplar*, not a candidate — it already has the shape.

---

## Do opportunistically, NOT as a scheduled wave

Right *direction*, wrong as a big up-front push — they'd be the wide tech-debt
surface that's explicitly not the goal.

- **`Player*` → actor/participant/viewpoint rename.** The taxonomy is correct and
  *is* the relativity principle. But there are hundreds of sites; renaming all now,
  justified largely by undesigned multiplayer, is speculative churn. **Rename
  role-by-role in files you're already editing for items 1–4. No `legacy`/alias
  module that doubles every name.** Preferred role mapping when you touch them:
  `PrimaryPlayer`→`PrimaryControlledActor` (or `LocalPresentationFocus` for
  camera/HUD), `PlayerEntity`→`ActorBody`/`ControlledActorMarker`,
  `PlayerInputFrame`→`ActorInputFrame`, `PlayerMana`/`PlayerCombatState`/
  `PlayerWallet`/`PlayerSafetyState`→`Actor*` equivalents,
  `PlayerDiedMessage`→`ActorDiedMessage` (+ source/cause).
- **fact/request/event message vocabulary.** Add the message seam when the
  *second* consumer appears, not preemptively.
- **Doc-consistency annotations.** Nine docs under `docs/planning`/`docs/systems`
  say "COMPLETE" while bridge vocabulary survives (verified — e.g.
  `non-player-centric-actor-unification.md`, `monolith-next-batch.md`). Relabel as
  "landed X, remaining Y" alongside the code they describe; don't let it *gate*
  engineering.

---

## Deliberately deferred / avoid

- **Bridge/alias/compat scaffolding.** No external consumers, no release → no
  compat tax. Delete-and-fix beats two-step migration here.
- **The mutable-authority world pole.** Avoid. Authored-`RoomGeometry` +
  derived-collision-view is the model. Only *persistent* mid-room geometry change
  could pull toward mutability — see open questions.
- **Crate-splitting `ambition_content`.** Do module families first; split into
  crates only when a boundary proves itself.
- **Choosing netcode.** Prepare seams (actor/source/cause attribution; item
  instance identity + holder) compactly so causality exists for future
  replay/multiplayer — but do not pick or build a netcode implementation.

---

## Open questions

1. **Falling sand is the forcing function for the world model.** If settled sand
   becomes *durable* collision (`falling_sand.rs`), the pure derived-view model
   needs a **durable overlay tier** (a persistent block list that survives frames
   but still isn't the authored base) rather than a mutable authoritative world.
   Verify how settled sand reaches collision before committing the "no persistent
   mutation" stance. This is the one fact that confirms or complicates the
   RoomGeometry decision.
2. **`GameWorld` authority vs. derived** — resolved (RoomGeometry + collision view),
   pending only the falling-sand check above.
3. **Exact new crate names** for domain plugins — pick at extraction time.
4. **How far to split `ambition_content`** into crates vs. module families.
5. **`ae::World` → `Terrain`?** Optional, lower priority than the resource rename.
6. **Future quest/progression model** — preserve facts/save boundaries; don't
   design around today's quest code.

---

## Search entry points (for a fresh agent)

```text
Input/control:    ambition_input/src/control.rs · gameplay_core/src/player/{components/mod.rs,systems.rs}
                  · app/src/app/{plugins.rs,sim_systems.rs}
Actor/brain:      characters/src/{actor,brain} · gameplay_core/src/{player,features}
Carryable items:  gameplay_core/src/items/pickup/mod.rs · content/src/items
                  · characters/src/brain/action_set/mod.rs
Combat/projectiles: ambition_combat · gameplay_core/src/{combat,projectile,enemy_projectile}
                  · app/src/app/combat_schedule.rs
World/rooms/LDtk:  gameplay_core/src/world{,/ldtk_world} · app/src/app/{world_flow,progression_schedule.rs}
Portal exemplar:   ambition_portal · ambition_portal_presentation · content/src/portal
Cutscenes/dialogue: ambition_cutscene · render/src/cutscene · content/src/dialogue · gameplay_core/src/dialog
Collision dedup:   engine_core/src/movement/collision.rs · platformer_primitives/src/{kinematic.rs,world_query.rs}
```

---

## Guiding contracts for patches

```text
RoomGeometry is authored, swapped at room boundaries; collision is read through
  the composited view, never the bare geometry.
Simulation is modelled around actors and actor-local intent, not a global input
  frame or a primary player.
Carryable items stay one lifecycle across held/world/thrown/recovered.
Attach source/cause attribution to projectiles/damage/effects/SFX/facts compactly.
Named content (spells, bosses, rooms, cues) lives in content, not foundation crates.
Reusable mechanics are Bevy plugins with owned resources/messages/local sets and
  explicit extension points; the app composes and hosts, it does not define meaning.
Copy the portal shape to extract a domain: runtime core, optional presentation,
  optional content/adapter package, host schedule mapping.
Canonical import path per concept; bridge vocabulary, if unavoidable, is named
  legacy/adapter/compat and is temporary.
Delete, don't bridge. Rename in place, don't alias. Add seams when the second
  use case lands.
```
