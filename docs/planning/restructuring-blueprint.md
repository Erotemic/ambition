# Restructuring blueprint — actionable distillation

*Author: Claude Opus 4.8 (1M) · 2026-06-25 · status: IN PROGRESS — see the Status log. §1 (shims), §2 (collision dedup + collision-view API, bar the portal reader), and most of §3 (app-drain) are done and feel-confirmed. **Remaining work: the portal shot-placement reader (a design fork), gnu_ton's subtractive arena gate (needs an overlay-model extension), §4 (ControlFrame → actor-local intent), and falling_sand → overlay (§5 OnceLock classification landed 2026-06-26).**  All landed feel-tests have been verified by Jon (2026-06-25).*

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

## Status log — what is NEXT (2026-06-25)

A fresh agent should read this first. The remaining work is listed up top; the
landed record (compressed) follows for context. Each item links to its section.

### Remaining work (the actionable list)

1. **Portal shot-placement adapter** (`content/portal/shot_adapter.rs`) — the last
   genuine collision reader still on bare `RoomGeometry`. Routing it onto the
   `CollisionWorld` view is a **design fork, not a sweep**: decide whether a portal
   may be placed on a moving platform / ECS solid (and whether carving the aperture
   into the world the shot raycasts against creates feedback). Decide that first;
   add a test. (§2)
2. **gnu_ton's subtractive arena gate** (`content/bosses/gnu_ton.rs`) — the last
   mid-room base mutator. It *removes* authored Ladder `climbable_regions` (stashed
   on entry) and a floor-gate Solid block on defeat, relying on room-reload to
   restore. The landed `FeatureEcsWorldOverlay::gate_solids` is additive-only, so
   this needs an overlay-model extension: a way to *subtract* authored blocks (a
   carve, like portals do) AND a climbable-region overlay (the overlay models
   neither water nor climbable regions today). Invert cleanly under an immutable
   base: the base always carries the ladders + floor-gate; the overlay removes them
   based on gate state. (§3 world / write-map note below)
3. **§4 ControlFrame → actor-local intent** — untouched. ~46 systems read global
   `Res<ControlFrame>`; move *simulation* onto entity-local `ActorIntent`. See §4.
4. **falling_sand → overlay** (Open question #1) — the other per-frame
   `RoomGeometry` writer; behind a feature flag. Wants *additive* overlay sand
   blocks, so it can likely push to `gate_solids` (or a sibling additive field), no
   subtractive extension needed (unlike gnu_ton).

**Remaining projectile/combat attribution work:** the generic/named projectile
split is done, but two attribution seams are still open — replacing the
player/enemy projectile split with source/faction, and entity-tracking
enemy-projectile owners (`EnemyProjectile` is string-`ProjectileOwnerId`-owned, so
its `DeathCause.attacker` is honestly `None` until the owner `Entity` is threaded
through the effect/spawn pipeline). (§"Projectiles / combat")

Guard when extending the collision-view: do NOT route render/layout/metadata
readers or projectiles (projectiles pass through platforms → `carves_only`) — a
reader that suddenly sees moving-platform / carve geometry is a silent feel
regression.

### Landed record (compressed — committed to `main`, feel-confirmed)

- **All 7 compat shims removed** (§1). One canonical import path per concept;
  `architecture_boundaries` guards against re-adding them.
- **Collision-semantics dedup + drift unification** (§2). `ambition_engine_core::
  collision_semantics` is the single kernel; `movement::collision` and
  `platformer_primitives::kinematic` both delegate.
- **`GameWorld` → `RoomGeometry` rename** (155 sites) + **`CollisionWorld`
  collision-view API** (`combat::world_overlay`: `solids()`/`carves_only()`/
  `base()`). Traversal abilities, body-mode clearance, and dropped-item physics
  routed off bare `RoomGeometry` onto the view.
- **§3 app-drain — substantially landed.** Where the drained code now lives:
  `detect_room_transition_system` → `gameplay_core::rooms`; attack-phase machine +
  `attack_advance_system` → `gameplay_core::combat::attack`; victim-side damage +
  `apply_player_hit_events` → `gameplay_core::combat::damage`; room-load sim half →
  `gameplay_core::rooms::load_room_geometry` (render tail stays in app's
  `load_room`); movement-event Sfx/Vfx → `gameplay_core::player::movement_fx`;
  `PlayerDiedMessage` → `ActorDiedMessage`. `app/world_flow.rs` is now host glue
  and the whole `ambition_app::app` module is explicit-import (no `use super::*`
  globs, no module-level `allow(unused_imports)` — do not reintroduce them).
- **`BossSpecialContentPlugin`** — the 11 named boss-special Techniques + cut-rope
  flavor drained out of `app/combat_schedule.rs` onto a new `CombatSet {
  ContentSpecials, ContentFlavor }` extension-slot enum; the app configures only
  WHERE each slot sits, content plugins own the systems.
- **`ActorDiedMessage` `DeathCause { source: HitSource, attacker: Option<Entity> }`
  attribution** — threaded from the killing `HitEvent`; reuses `HitSource`.
  Attacker entity stamped on `BossAttack`/`BossBody`/`EnemyBody`/`EnemyChargeCrash`.
- **Generic-vs-named projectile split.** Foundation `platformer_primitives` is now
  generic (`bounces: u8` field instead of `match kind`; `ProjectileKind`/stat
  tables/serde dep removed); the named kit lives in
  `gameplay_core::projectile::kind` (`ProjectileKind::spec()`/`charged_spec()`);
  `ProjectileKind` rides as its own ECS `Component` on player shots.
  `architecture_boundaries` asserts the foundation projectile module names no
  `ProjectileKind`/`Fireball`/`Hadouken`.
- **World gate→overlay, 2 of 3.** `FeatureEcsWorldOverlay::gate_solids`:
  authored-equivalent static solids composited into every base-reader (player/
  traversal/item collision via `CollisionWorld::solids`, projectile collision via
  `world_with_gate_solids_and_carves`, and the render `LockWallVisual` reconcile).
  Encounter lock walls + intro flag gates now derive onto `gate_solids` instead of
  mutating the base; neither system takes `ResMut<RoomGeometry>` anymore.
- **Falling-sand world-model question RESOLVED** — confirms the RoomGeometry model;
  no durable-overlay tier needed (Open questions #1).

> Orientation — the `ResMut<RoomGeometry>` writer classification (verify against
> `main` before acting):
> - **Boundary swaps (legitimate — authored base, write-once-per-room):**
>   `session/reset/mod.rs` (reset), `app/world_flow/room_flow.rs` (transition apply
>   — the geometry swap itself now lives in `gameplay_core::rooms::load_room_geometry`),
>   `app/dev_runtime.rs` (hot-reload). These are NOT base-immutability violations.
> - **Mid-room mutators (the violations):** `encounter/systems.rs` (lock walls) and
>   `content/intro/route_state.rs` (flag gates) — **both DONE** (now `gate_solids`
>   overlay contributors, no longer take `ResMut<RoomGeometry>`).
>   `content/bosses/gnu_ton.rs` (arena gate) — **REMAINS**, subtractive; needs the
>   overlay-model extension described above. `falling_sand.rs` snapshots its own base
>   and re-projects per frame (Open question #1) — convert to an overlay contributor
>   like the gates, but it needs additive sand blocks, not gate solids.

---

## Structural-work feel-risk (for the remaining steps)

**Working stance (Jon):** structural refactors may land *ahead* of feel/visual
verification; feel regressions get fixed afterward and are not a blocker. So
**nothing here gates further structural work.** All feel-tests owed by the landed
work (collision drift, collision-view batch, lock-wall draw, projectile
generic/named split) were **verified by Jon on 2026-06-25** and are cleared.

Where to expect owed feel/visual checks on the *remaining* steps (clear them in a
play session; if one feels wrong, revert or patch the *named commit* — cheap, the
changes are isolated). Headless tests cover correctness; what a headless test
cannot see (sprite draw, on-screen feel, timing) is what's owed.

| Step | Feel risk | Notes |
| --- | --- | --- |
| Content module families reorg | **none** | module moves only |
| gnu_ton arena gate → overlay | **low** | collision headless-testable (ladder/floor-gate present-or-not); needs the subtractive overlay extension |
| §4 ControlFrame → actor-local intent | **low–med** | behaviour-preserving by design; input is feel-y, so any drift shows → owed input check |
| Audio plugin split (backend/director/cue) | **low–med** | playback should be unchanged if careful → owed audio check |
| falling_sand → overlay | **med** | sand collision projection; standing/climbing settled sand piles (behind its feature flag, low exposure) |

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

> **Status:** done except the portal shot-placement reader (a design fork) — see
> the Status log. The `CollisionWorld` read-API exists and the collision readers
> are routed onto it; the section below is retained as the design rationale.

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
3. **The raw `Res<RoomGeometry>` collision readers are the bug.** They read bare
   geometry when they should read the collision view. Promote the composite to the
   single collision read-API and route the *collision* readers through it (NOT the
   metadata/layout ones). This is the *same seam* the collision-semantics dedup
   needs (plan item 2) — one frontier. Post-rename write-map (43 readers / 7
   writers, with the metadata-vs-collision split) is in the **Status log** above.

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

### 1. Delete the compatibility shims (one canonical import per concept) — DONE

All 7 `ambition_gameplay_core::lib.rs` re-export shims (`kinematic`, `ui_nav`,
`interaction`, `actor`, `brain`, `engine_core`, `input`) removed; canonical
imports everywhere; the `architecture_boundaries` test now *guards against
re-adding* them (inverted assertion). Validation: `rg
"ambition_gameplay_core::(input|engine_core|brain|actor|interaction|ui_nav|kinematic)"
crates` → zero internal hits.

### 2. Collision/support-semantics dedup (+ RoomGeometry collision view)

The semantics kernel is extracted (`ambition_engine_core::collision_semantics`,
both sweeps delegate) and the collision-view API is routed (see Status log). Two
items remain:

- **The portal shot-placement reader** (the collision-view design fork — Status
  log item 1).
- **Deferred:** decide whether controlled-body movement consumes `step_kinematic`
  directly or keeps a richer sweep over the same kernel. Only worth doing if the
  two sweeps (`engine_core/src/movement/collision.rs` controlled-body vs
  `platformer_primitives/src/kinematic.rs` generic actor) start drifting again;
  parity holds today.

### 3. Drain simulation out of `ambition_app` into domain plugins

The app should compose and host, not define domain meaning (see app contract).
The first-movers, `app/world_flow` drains, `BossSpecialContentPlugin`,
`ActorDiedMessage` attribution, the projectile generic/named split, and the
encounter + intro lock-wall gate→overlay conversions all **landed** — see the
Status-log "Landed record" for where the code now lives. `app/world_flow.rs` is
host glue and the `app` module is fully explicit-import.

What STILL remains:

- World: **gnu_ton's subtractive arena gate** — the last mid-room base mutator;
  needs an overlay-model extension (authored-block carve + climbable-region overlay)
  beyond what additive `gate_solids` covers. `falling_sand.rs` is the other
  per-frame base writer (Open question #1).
- Combat: the remaining projectile/combat attribution seams — replace the
  player/enemy projectile split with source/faction, and entity-track
  enemy-projectile owners (see the Status-log remaining list).

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

### 5. Classify the `OnceLock` global registries — DONE (2026-06-26)

Eight `OnceLock`s, classified inline at each `static` (search `§5 classification`).
They split into exactly two populations:

**Content registries (install-once seam — 5):** `BOSS_PROFILE_OVERRIDE`,
`BOSS_SPECIAL_ANIM_KEYS` (`boss_encounter/behavior.rs`),
`BOSS_ENCOUNTER_SPEC_OVERRIDE` (`boss_encounter/specs.rs`), `ENCOUNTER_WAVE_BOOK`
(`encounter/loading.rs`), `ENEMY_ROSTER_OVERRIDE` (`features/enemies/mod.rs`).
Each is installed by `ambition_content` at plugin-build time, immutable after,
and read from a **pure resolution helper** with no `World` access
(`BossBehaviorProfile::from_data`, `boss_encounter_specs`,
`authored_encounter_waves`, `spec_for_brain`). The `install_*` fn + the
`cfg(test)` fixture together ALREADY ARE the test-override seam.

**Immutable asset caches (3):** `file_root_registry` / `player_render_size` SPEC
(`character_sprites/attack_hitbox.rs`), `record_index`
(`character_sprites/sheets/mod.rs`). Each is derived once from the compile-time
`BAKED_SHEET_RONS` table; pure, override-free, no install seam.

**Decision — the v5 "promote content registries toward resources" is REJECTED on
inspection.** The reads are all in pure, non-system spawn/profile helpers
(`spawn_enemy`, `BossProfile::from_id`, the LDtk encounter loader). A Bevy
`Resource` would force threading `Res<…>`/`&World` through that pure spec/spawn
layer — a wide ECS-coupling surface for zero behavioral gain, the exact tech-debt
spread the narrow-types principle warns against. The install-once `OnceLock` +
typed registry + `cfg(test)` fixture IS the elegant shape for "content data
installed at startup, immutable after, resolved by pure functions." The work done
here is the classification itself (made explicit + uniform in code so it isn't
re-litigated), plus naming the sheet caches as asset caches per the original ask.
If a future use case needs per-`World` content variants (e.g. multiplayer with
divergent content packs), revisit then — that's the "add the seam when the second
use case lands" trigger, not now.

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
- **Landed (2026-06-25):** movement-event Sfx/Vfx emission → `player::movement_fx`.
  The role-audit + actor-runtime plugin remain.

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
- **Now:** `ambition_combat` has primitives; the foundation `platformer_primitives`
  is now generic (named-kind leak closed); the named kit lives in
  `gameplay_core::projectile::kind` with `ProjectileKind` riding as an ECS
  component; gameplay_core still has a `projectile`/`enemy_projectile` split by
  player/enemy.
- **Target:** `ProjectileRuntimePlugin` (generic body/lifecycle/messages + source
  actor/item/faction/authority-tick) + `CombatRuntimePlugin` (hitbox/damage/facts/
  attribution) + content plugins for named projectile kinds.
- **Landed (2026-06-25):** attack-phase machine + `attack_advance_system` →
  `combat::attack`; victim-side damage + `apply_player_hit_events` →
  `combat::damage`; `BossSpecialContentPlugin` (specials + cut-rope onto `CombatSet
  { ContentSpecials, ContentFlavor }`); `ActorDiedMessage` `DeathCause { source,
  attacker }`; the generic/named projectile split (foundation generic, named kit in
  `gameplay_core::projectile::kind`).
- **First move (what remains):** replace the player/enemy projectile split with
  source/faction; entity-track enemy-projectile owners (`EnemyProjectile`'s
  `DeathCause.attacker` is honestly `None` until the owner `Entity` threads through
  the effect/spawn pipeline). Keep the projectile-spawn timing contracts
  (comments → tests).

### World / rooms / LDtk
- **Now:** `RoomGeometry` (was `GameWorld`) is the per-room geometry; `world/
  ldtk_world` owns conversion/runtime; app detects/applies room transitions. The
  mid-room base-mutating gates are now overlay contributors (below); only gnu_ton's
  subtractive arena gate still mutates the base directly.
- **Target:** `WorldRuntimePlugin`/`LdtkWorldPlugin` owns load/reset/lifecycle +
  the RoomGeometry contract + the collision-view read API; content mutates world
  through the collision overlay (or explicit commands/facts), not ad hoc base edits.
- **Landed (2026-06-25):** `detect_room_transition_system` → `rooms`; room-load
  split — sim half → `rooms::load_room_geometry`, render tail stays in the app's
  `load_room`. **Write-map classified** (Status-log orientation note).
  **Encounter + intro lock walls converted** off direct `RoomGeometry` mutation
  onto the new `FeatureEcsWorldOverlay::gate_solids` overlay category (composited
  into player/projectile collision + the render `LockWallVisual` reconcile).
- **First move (what remains):** convert gnu_ton's subtractive arena gate — needs
  an overlay-model extension (authored-block carve + climbable-region overlay) that
  additive `gate_solids` does not cover. Then `falling_sand.rs` (Open question #1).
  A formal world command/event vocabulary is only needed if a gate wants more than
  the overlay expresses.

### Content / adapters
- **Now:** `ambition_content` mixes authored rows, install plugins, and runtime
  adapters; portal adapters still use global `ControlFrame` + gameplay-core shims.
  (The former `ambition_render::cutscene` runtime leak is closed — see Cutscenes.)
- **Target:** module families `content::{authored, install, adapters,
  presentation_bindings}`; adapters are explicit domain→domain translators mounted
  into extension sets.
- **First move:** split the module families (cheap conceptual reorg, not crate
  split); move portal/boss adapters into explicit adapter modules; treat quest as
  facts/commands only.

### Cutscenes / dialogue / render — **DONE (2026-06-25)**
The render→content cutscene-runtime boundary leak is closed. Runtime types
(`CutsceneLibrary`, `RoomCutsceneBindings`) → `ambition_cutscene`; runtime systems
+ `CutsceneSchedulePlugin` → `ambition_gameplay_core::cutscene`; authored defaults
→ `ambition_content::dialogue::cutscene_defaults`; presentation
(`CutsceneOverlayRoot`, `sync_cutscene_ui`) stays in `render::cutscene`,
presentation-only. No compat shims — all consumers repointed to canonical paths.

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
5. ~~**Cutscene runtime plugin**~~ — **DONE (2026-06-25)**; see Cutscenes section.
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
  `PlayerWallet`/`PlayerSafetyState`→`Actor*` equivalents.
  (`PlayerDiedMessage`→`ActorDiedMessage` done 2026-06-25; source/cause still TODO.)
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

1. **Falling sand — RESOLVED (2026-06-25), confirms the decision.** Investigated
   `falling_sand.rs`: it snapshots the authored room into its own `base_blocks`,
   then every frame restores `world.0.blocks` to that base and re-projects settled
   particles as `OneWay` collision blocks (`falling_sand.rs:328, 878, 955`). The
   *durable* state is the particle simulation, NOT the collision world — collision
   is a per-frame derived projection of it, exactly the derived-view pattern. So
   **no durable-overlay tier and no mutable-authoritative world are needed.** Under
   RoomGeometry, falling sand drops its private base-snapshot (reads the immutable
   `RoomGeometry`) and contributes sand blocks to the overlay like portal carves —
   *simpler* than today's clobber-and-restore. Migration note: with the encounter +
   intro lock-wall gates now converted to overlay contributors, falling sand (behind
   its feature flag) is the main remaining per-frame `RoomGeometry`-writer. The landed
   `FeatureEcsWorldOverlay::gate_solids` contributor pattern is the template — sand
   wants *additive* overlay blocks, so it can likely push to `gate_solids` (or a
   sibling additive field) directly, no subtractive extension needed (unlike gnu_ton).
2. **`GameWorld` authority vs. derived** — RESOLVED: RoomGeometry (authored base)
   + derived collision view over the overlay. The falling-sand check above removed
   the last open dependency.
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
