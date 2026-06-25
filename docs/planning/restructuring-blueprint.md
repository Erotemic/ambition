# Restructuring blueprint — actionable distillation

*Author: Claude Opus 4.8 (1M) · 2026-06-25 · status: IN PROGRESS — see the Status log. §1 (shims) and §2 (collision dedup + collision-view API, bar the portal reader) done; §3 (app-drain) substantially done — the three first-movers + world_flow drains, the `BossSpecialContentPlugin` (boss specials + cut-rope onto `CombatSet` slots), `ActorDiedMessage` `DeathCause` attribution, and 2-of-3 world gates (encounter + intro lock walls → `gate_solids` overlay) all landed. Still open in §3: the generic-vs-named projectile split (feel-critical, deferred) and gnu_ton's subtractive arena gate (needs overlay-model extension). §4, §5 untouched.*

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

## Status log — what has LANDED, and what is next (2026-06-25)

A fresh agent should read this first. Each item below links to the section with
the full reasoning; the section is annotated **DONE** / **NEXT** / **OPEN** inline.

**Landed (committed to `main`, all validated):**

- **All 7 compat shims removed** (§1 DONE). One canonical import path per concept.
  `kinematic`/`ui_nav`/`interaction` (earlier session) + `input` (`4e9743a2`),
  `engine_core` (`bf0daf5a`, `ae` alias kept), `actor` (`a4a63d73`), `brain`
  (`c9f647a5`). `architecture_boundaries` now *guards against re-adding* the shim
  (inverted assertion). Stale shim paths in docs swept (`6eb9b97d`, `2fa23bf0`).
- **Collision-semantics dedup + drift unification** (§2 DONE). New
  `ambition_engine_core::collision_semantics` is the single source of truth for
  the gravity-relative support/surface kernel; `movement::collision` and
  `platformer_primitives::kinematic` both delegate. Kernel extract `39313f26`,
  drift unification `c732671c`. The 3 former drifts are unified on canonical rules
  (1px `EDGE_OVERLAP_SLOP`, zero-gravity one-way guard, `body_on_support_side`).
  FEEL-TEST owed: kinematic enemies now treat platform edges 1px more strictly.
- **`GameWorld` → `RoomGeometry` rename** (§"Resolved decision" DONE). Pure
  identifier rename, 155 sites, `90966244`. Names what it *is* (authored room
  geometry), not what it isn't.
- **`CollisionWorld` collision-view API** (§2/§"Resolved decision" — seam `b51ac38e`,
  readers `0208bbb5`). Single composited collision read-API; traversal abilities,
  body-mode clearance, and dropped-item physics routed off bare `RoomGeometry`.
  FEEL-TEST owed (blind batch); portal shot-placement reader still pending (a
  design fork — see below).
- **Falling-sand world-model question RESOLVED** (`25fcb13a`) — confirms the
  RoomGeometry model; no durable-overlay tier needed (see Open questions #1).

**RoomGeometry collision-view API — mostly done; what remains:**

The `CollisionWorld` read-API exists (`combat::world_overlay`, via `features`):
`solids()` / `carves_only()` / `base()`. The traversal abilities, body-mode
clearance, and dropped-item physics are routed onto it; the boss-special /
encounter readers were classified as bounds-metadata (room `size`, not collision)
and correctly stay on the base. Two things are left:

1. **Portal shot-placement adapter** (`content/portal/shot_adapter.rs`) is the
   last genuine collision reader still on bare `RoomGeometry`. Routing it is a
   **design fork, not a sweep**: deciding whether a portal may be placed on a
   moving platform / ECS solid (and whether carving the aperture into the world
   the shot raycasts against creates feedback). Decide that first; add a test.
2. **Feel-check owed** on the routed batch (`0208bbb5`, blind): in a room with a
   moving platform, confirm grapple latches it, blink/dive stop on it, and
   unmorph is blocked under it. Revert that one commit if any feels wrong.

Guard when extending: do NOT route render/layout/metadata readers or projectiles
(projectiles pass through platforms → `carves_only`) — a reader that suddenly
sees moving-platform / carve geometry is a silent feel regression.

**§3 app-drain — substantially landed (2026-06-25 session).** The three
first-movers drained out of `ambition_app` into runtime crates, and several more
followed. Where the drained code now lives (so a fresh agent doesn't hunt for it
in the app):

- `detect_room_transition_system` → `gameplay_core::rooms`.
- attack-phase machine + `attack_advance_system` → `gameplay_core::combat::attack`.
- victim-side damage resolution + `apply_player_hit_events` → `gameplay_core::combat::damage`.
- room load split: sim half → `gameplay_core::rooms::load_room_geometry` (returns
  a `RoomLoadResult`); render spawns + arrival VFX stay in the app's `load_room`.
- movement-event Sfx/Vfx emission → `gameplay_core::player::movement_fx`.
- `PlayerDiedMessage` → `ActorDiedMessage` (the death-fact vocabulary).
- **`BossSpecialContentPlugin`** (`1bdb0cf7`'s parent `5a6f02f3`): the 11 named
  boss-special Techniques + cut-rope flavor drained out of `app/combat_schedule.rs`
  onto a new `CombatSet { ContentSpecials, ContentFlavor }` extension-slot enum
  (mirrors `BossSteerSlot`). The app now configures only WHERE each slot sits in
  the combat chain; the content plugins own the systems. State-component
  registration moved with the specials into the new plugin.
- **`ActorDiedMessage` source/cause attribution** (`1bdb0cf7`): added
  `DeathCause { source: HitSource, attacker: Option<Entity> }`, threaded from the
  killing `HitEvent` at the single death choke point. Reuses `HitSource` (no
  parallel enum). The compact causality seam, landed ahead of any consumer.

`app/world_flow.rs` is now just host glue (`RoomClock`, `sandbox_dt`,
`ground_gap_below_feet`, the `room_flow` submodule). The whole `ambition_app::app`
module is now explicit-import — the `use super::*`-family glob blocks and the
module-level `allow(unused_imports)` are gone (a navigability win; do not
reintroduce them).

**Still open in §3+:** two deeper extractions remain, both larger/riskier than
what has landed:

1. **Generic-vs-named projectile split (combat) — DONE (2026-06-25).** The verified
   leak (`ProjectileKind { Fireball, Hadouken, HadoukenSuper }` + stat tables in the
   foundation crate `platformer_primitives`) is closed. What landed:
   - **Foundation made generic.** `ProjectileSpec` / `ProjectileGameplay` carry only
     generic data; `match kind` for the bounce budget became a `bounces: u8` field
     (`from_spec` reads `spec.bounces`). `ProjectileKind`, `FireballChargeTuning`,
     the stat tables, `ProjectileSpec::new(kind,..)` and `with_charge_tier` were all
     removed from `platformer_primitives` (its now-unused `serde` dep dropped).
   - **Named kit moved UP to `gameplay_core::projectile::kind`** (not content — the
     player fire/charge/gesture systems that consume it can't reach content without
     the §4 ControlFrame extraction; gameplay_core is "machinery" and a strict
     improvement over the foundation leak). `ProjectileKind::spec()` lowers a kind
     into the generic spec; `charged_spec()` applies the fireball tier ramp.
   - **Kind rides as its own ECS component.** `ProjectileKind` is now a `Component`
     attached to player shots at spawn (via a new `SpawnProjectile.kind` field).
     `step_projectiles` queries `Option<&ProjectileKind>` for `HitSource` /
     trace; render reads it for tint + sprite name. The engine body never names a
     kind. Trace `Hit`/`Expired` now carry `Option<ProjectileKind>` (enemy shots are
     genuinely kind-less → label "projectile" instead of the old fake "fireball").
   - **Guard added.** `architecture_boundaries` now asserts the foundation projectile
     module names no `ProjectileKind`/`Fireball`/`Hadouken` (regression fence).
   All projectile/collision/charging/portal/arch tests green. Owed: Feel-test #4.
   **Attacker-entity attribution — DONE (2026-06-25).** `HitEvent.attacker` is now
   stamped on hostile sources wherever an entity exists: `BossAttack`/`BossBody`
   (boss entity, added to the bosses query), `EnemyBody`/`EnemyChargeCrash` (enemy
   entity). It flows into the victim's `DeathCause`. Honest `None` remains for
   `Hazard` (environmental) and `EnemyProjectile` (string-`ProjectileOwnerId`-owned,
   not entity-tracked — threading the owner `Entity` through the effect/spawn pipeline
   is a separate step). **Still remaining in this domain:** replacing the player/enemy
   projectile split with source/faction, and entity-tracking enemy-projectile owners.
2. **World gate→overlay (world) — 2 of 3 gates DONE (2026-06-25).** The fork was
   resolved toward the authored-base preference: gates contribute to the per-frame
   collision overlay, not the base. New overlay category
   `FeatureEcsWorldOverlay::gate_solids` (`4adf73f2`): authored-equivalent static
   solids composited into EVERY base-reader (player/traversal/item collision via
   `CollisionWorld::solids`, projectile collision via the new
   `world_with_gate_solids_and_carves`, AND the render `LockWallVisual` reconcile)
   — so a lock wall collides and draws exactly as a base wall while the base stays
   immutable. The **encounter lock walls** (`4adf73f2`) and **intro flag gates**
   (`3b26e7fb`) now derive onto `gate_solids` in WorldPrep contributors;
   `update_encounters_from_world` and `sync_intro_flag_gated_lock_walls` no longer
   take `ResMut<RoomGeometry>`. All headless-tested (gate solids in player + projectile
   views; render reconcile; pure-fn derive).
   **Remaining: gnu_ton's arena gate** (`content/bosses/gnu_ton.rs`) — the last
   mid-room base mutator. It is *subtractive*: it removes authored Ladder
   `climbable_regions` (stashed on entry) and a floor-gate Solid block on defeat,
   relying on room-reload to restore. `gate_solids` is additive-only, so this needs
   an overlay-model extension — a way to subtract authored blocks (a carve, like
   portals do) AND a climbable-region overlay (the overlay models neither water nor
   climbable regions today). Invert cleanly under an immutable base: the base always
   carries the ladders + floor-gate; the overlay *removes* them based on gate state.
   The geometry-swap writers (`load.rs`, `reset/mod.rs`, `dev_runtime.rs`) and
   `falling_sand.rs` are classified in the write-map below and are NOT base-immutability
   violations (boundary swaps / a system that snapshots its own base).

§4 (ControlFrame→actor-local intent) and §5 (OnceLock classification) are untouched.

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

## Feel-test checklist (owed verification) + structural-work feel-risk

**Working stance (Jon, 2026-06-25):** structural refactors may land *ahead* of
feel/visual verification; feel regressions get fixed afterward and are not a
blocker. So **nothing below gates further structural work.** This is the running
ledger of owed visual/feel confirmations — clear them in a play session and, if
one feels wrong, revert or patch the *named commit* (cheap; the changes are
isolated). Headless tests already cover the *correctness* of each; what is owed is
only the part a headless test cannot see (sprite draw, on-screen feel, timing).

**Owed now (already landed, headless-correct, feel/visual unconfirmed):**

1. **Collision drift, 1px stricter edge** (`c732671c`, §2). Kinematic enemies now
   treat platform edges 1px more strictly. Look: enemies walking to a platform lip
   don't teeter/fall differently than before.
2. **Collision-view routed batch** (`0208bbb5`, §2, blind). In a room with a moving
   platform: grapple latches it, blink/dive stop on it, unmorph is blocked under it.
3. **Lock-wall visual draw** (`4adf73f2` encounter, `3b26e7fb` intro — gate→overlay).
   Collision + the `LockWallVisual` ECS reconcile are headless-tested; the actual
   on-screen sprite draw is not. Look: trigger an encounter (or an intro flag gate)
   and confirm the seal wall still *draws* (it now sources from `gate_solids`, not
   the base). Minor: the wall now appears 1 frame later than before (negligible).

**Added to this checklist when the corresponding structural step lands:**

4. **Projectile generic/named split** (LANDED 2026-06-25) — the feel-critical one,
   now owed a play-session check: fireball arcs/bounce (2 bounces, then expire),
   charge-tier ramps (size 1.0/1.4/1.8×, damage 1/4/16×), Hadouken vs HadoukenSuper
   gesture timing, and projectile render tints (fireball warm-orange, Hadouken blue,
   Super deep-blue) + the per-kind sprite name. Headless tests cover the mechanics;
   what's owed is on-screen feel/tint. Revert/patch the named commit if any feels off.
5. **falling_sand → overlay** (when done) — standing/climbing settled sand piles
   (behind its feature flag, low exposure).

**Structural-work feel-risk classification** (what is safe to bulldoze vs what adds
to the checklist — none are *blocked*, this is just where to expect owed checks):

| Step | Feel risk | Notes |
| --- | --- | --- |
| §5 OnceLock classification | **none** | registry/resource reorg, no behavior |
| ~~Cutscene runtime → `ambition_cutscene`~~ | **DONE** | the "bounded win" — landed 2026-06-25; runtime types→`ambition_cutscene`, systems→`gameplay_core::cutscene`, authored defaults→content, presentation stays in render |
| Content module families reorg | **none** | module moves only |
| ~~Attacker-entity attribution on enemy/boss `HitSource`s~~ | **DONE** | landed 2026-06-25; boss/enemy entity stamped on `HitEvent.attacker` → `DeathCause`. Hazard + enemy-projectile stay honest `None`. |
| gnu_ton arena gate → overlay | **low** | collision headless-testable (ladder/floor-gate present-or-not); needs the subtractive overlay extension |
| §4 ControlFrame → actor-local intent | **low–med** | behaviour-preserving by design; input is feel-y, so any drift shows → adds an owed input check |
| Audio plugin split (backend/director/cue) | **low–med** | playback should be unchanged if careful → owed audio check |
| falling_sand → overlay | **med** | sand collision projection; adds checklist #5 |
| ~~**Projectile generic/named split**~~ | **DONE** | landed 2026-06-25; foundation now generic, named kit in `gameplay_core::projectile::kind`. Owed feel-check #4 below. |

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

### 1. Delete the compatibility shims (one canonical import per concept)

> **DONE (2026-06-25).** All 7 shims removed; canonical imports everywhere; the
> `architecture_boundaries` test guards against re-adding them. The original plan
> and call-site table below are kept as the record of how it was done.

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
The three first-movers and the `app/world_flow` drains **landed (2026-06-25)** —
see the Status log for where the code now lives. `app/world_flow.rs` is host glue
and the `app` module is fully explicit-import.

The deeper extraction is mostly landed too (2026-06-25 — see the Status log for
detail). What is DONE here:

- Combat: **`BossSpecialContentPlugin` DONE** — boss specials + cut-rope mounted on
  the new `CombatSet { ContentSpecials, ContentFlavor }` extension slots.
  **`ActorDiedMessage` `DeathCause` attribution DONE.** `combat::{attack, damage}`
  are the runtime home.
- World: **encounter + intro lock walls DONE** — converted off direct
  `RoomGeometry` mutation onto the new `FeatureEcsWorldOverlay::gate_solids` overlay
  category (the write-map classification is in the Status-log orientation note).

What STILL remains (the two larger/riskier items):

- Combat: **split generic projectile stepping from named kinds** — feel-critical,
  6-crate, §4-entangled, but **safe to do structurally now** (land-then-feel-test;
  adds Feel-test checklist #4). See Status-log item 1. The remaining attribution
  work (attacker entity on enemy/boss-side `HitSource`s) rides with it.
- World: **gnu_ton's subtractive arena gate** — the last mid-room base mutator;
  needs an overlay-model extension (authored-block carve + climbable-region overlay)
  beyond what additive `gate_solids` covers. `falling_sand.rs` is the other
  per-frame base writer (Open question #1).

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
- **Landed (2026-06-25):** the player attack-phase machine + `attack_advance_system`
  → `combat::attack`; victim-side damage resolution + `apply_player_hit_events`
  → `combat::damage`. **`BossSpecialContentPlugin` DONE** — the 11 boss-special
  consumers + cut-rope flavor pulled out of `CombatSchedulePlugin` onto the new
  `CombatSet { ContentSpecials, ContentFlavor }` extension slots (in
  `gameplay_core::schedule`). **`ActorDiedMessage` `DeathCause { source, attacker }`
  attribution DONE.**
- **First move (what remains):** split generic projectile stepping from named kinds
  (`ProjectileKind` Fireball/Hadouken leak in `platformer_primitives` — feel-critical
  + 6-crate, but **safe to land structurally now** then feel-test; Feel-test checklist
  #4, Status-log item 1); add attacker-entity attribution to enemy/boss-side
  `HitSource`s; replace the player/enemy projectile split with source/faction. Keep
  the projectile-spawn timing contracts (comments → tests).

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
  adapters; **it imports `ambition_render::cutscene` runtime resources (verified
  boundary leak)**; portal adapters still use global `ControlFrame` + gameplay-core
  shims.
- **Target:** module families `content::{authored, install, adapters,
  presentation_bindings}`; adapters are explicit domain→domain translators mounted
  into extension sets.
- **First move:** split the module families (cheap conceptual reorg, not crate
  split); move portal/boss adapters into explicit adapter modules; treat quest as
  facts/commands only.

### Cutscenes / dialogue / render — a clean bounded win — **DONE (2026-06-25)**
- **Now (was):** `ambition_cutscene` existed but cutscene runtime types
  (`CutsceneLibrary`, `ActiveCutscene`, `CutsceneTriggerQueue`,
  `CutsceneAdvanceRequest`, `RoomCutsceneBindings`, `CutsceneSchedulePlugin`) lived
  in `ambition_render::cutscene`; content inserted those render resources.
- **Target:** `ambition_cutscene` owns runtime vocab; the playback systems own the
  drive; `render::cutscene` owns UI/render only; content installs scripts/bindings
  into the *runtime*, not render.
- **Landed (2026-06-25):** classified every type/system and split by the actual
  dependency layering (`gameplay_core` → `ambition_cutscene`, so the
  gameplay-coupled systems can't sink into the foundation crate):
  - **Runtime types** `CutsceneLibrary` + `RoomCutsceneBindings` (pure, content-free)
    → `ambition_cutscene` (joins `ActiveCutscene`/`CutsceneAdvanceRequest`/the
    script+stepper already there).
  - **Runtime systems + plugin** (`auto_trigger_room_cutscenes`,
    `drain_cutscene_triggers`, `tick_active_cutscene`, `CutsceneSchedulePlugin` —
    need `RoomSet`/`SandboxSave`/`SandboxSet`/`CutsceneTriggerQueue`) →
    new `ambition_gameplay_core::cutscene` (sibling of `cutscene_trigger`).
  - **Authored defaults** (`default_cutscene_library`, the room→cutscene bindings —
    named Ambition scripts/rooms) → `ambition_content::dialogue::cutscene_defaults`
    (`RoomCutsceneBindings::defaults()` → free fn `default_room_cutscene_bindings()`).
  - **Presentation** (`CutsceneOverlayRoot`, `sync_cutscene_ui`) → stays in
    `render::cutscene`, now presentation-only.
  - Net: **content no longer imports `ambition_render` for cutscene runtime** (the
    verified boundary leak is closed); the only remaining `render::cutscene`
    references are the presentation overlay + accurate doc comments. No compat
    shims/re-exports — all consumers repointed to canonical paths. Feel-risk: none
    (type moves only; build + all cutscene/arch-boundary tests green).

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
5. ~~**Cutscene runtime plugin** — move runtime out of render (the bounded win).~~
   **DONE (2026-06-25):** runtime types → `ambition_cutscene`, playback systems +
   `CutsceneSchedulePlugin` → `ambition_gameplay_core::cutscene`, authored defaults →
   `ambition_content::dialogue::cutscene_defaults`, presentation stays in
   `render::cutscene`. Content/render cutscene-runtime leak closed.
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
