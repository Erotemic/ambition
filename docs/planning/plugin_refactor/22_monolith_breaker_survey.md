# Stage 21 — Monolith-breaker survey + move-up work (2026-06-10, Opus)

Continuation after the Stage 20 bisection (`21_stage20_attack_plan.md`). The
bisection cut the crate *graph* (machinery lib ← `ambition_content` ← `ambition_app`);
this stage attacks the **remaining bulk inside `ambition_sandbox`** (still ~112k LOC,
the lion's share of the workspace).

## The two kinds of breaker (this is the key mental model)

1. **Move UP to `ambition_app`** — the composition layer may import anything, so a
   module moves up cleanly **iff its lib-external consumers are few/zero** (or only
   need a small vocabulary slice you leave behind). Cheap, mechanical, strong
   verification. *No inversions needed.*
2. **Extract DOWN to a reusable crate** — must invert every content/named **and every
   upward-machinery** coupling first. Expensive (this is what B3 render is).

**Critical lesson (cost me two mis-ratings):** "content-free" (passes the
architecture guard) is **NOT** the same as "extractable." A module can name zero
content yet still depend on 15 sibling machinery modules — that blocks a DOWN
extraction just as hard. Always measure *outward* deps, not just *named-content* refs.

## Ranked candidates (measured 2026-06-10, against the ~112k lib)

| Module | Lines | Best dir | Difficulty | Verified blocker |
|--------|------:|----------|-----------|------------------|
| **menu** | 12,320 | → app | **DONE** (3a3725e3) | 2 small knots; host stack moved, ~3k stays |
| features | 15,492 | → content | Hard | `EnemyConfig.archetype` knot (A2 deep half) |
| presentation | 10,376 | → crate (B3) | Hard | boss-asset map; **weak verification (visual)** |
| world | 9,354 | → crate | Hard | LDtk adapter + named rooms |
| brain | 8,734 | → crate | Medium | named boss-attack enum variants |
| boss_encounter | 5,527 | → split | Hard | named bosses ↔ generic runtime |
| **mechanics** | 5,187 | → crate | **Hard (NOT easy)** | ~15 upward lib deps — see below |
| **dev** | 4,902 | → app (partial) | Mixed | only ~1.3k cleanly movable — see below |
| abilities | 3,661 | → content/crate | Medium | player ability content |
| projectile | 2,774 | → crate | Medium | (platformer_runtime already has a projectile/) |
| dialog | 2,073 | → content | Easy-med | triaged thin earlier |
| combat | 979 | → crate | Easy | damage primitives |

### menu — DONE (commit 3a3725e3)
Moved the **host stack (~9.3k)** up to `ambition_app::menu`: `model`, `dispatch`,
`effects`, `grid_backend` (2.2k), `kaleidoscope_app` (4.4k), `parity_tests`.
**Lib keeps `crate::menu` ≈ 3k** — the genuinely lib-coupled pieces:
- `ir/` (settings IR) — bidirectionally tied to `persistence::settings::model`.
- `map/` — the Map tab; `presentation::rendering` calls `handle_map_menu_hotkeys` and
  app reads `MapMenuState`.
- `backend.rs` (NEW) — `InventoryUiBackend` + `*_BACKEND_ENABLED` consts, carved out
  of `kaleidoscope_app` because `map`/`ir` (lib) read the selector. Methods made `pub`.

Guard: `architecture_boundaries_lib_menu_keeps_only_the_coupled_pieces`.

### mechanics — verified HARD, do NOT attempt as a quick crate
`mechanics/` (combat kit + gravity) is **content-free (guard-clean) but not
dependency-clean**. Measured outward deps: player(37), interaction(33), physics(22),
actor(22), portal(13), brain(13), features(11), rooms(10), combat(8), audio(8),
presentation(7), encounter(7), quest(6), items(6), world(5), boss_encounter(2),
abilities(1). Combat hitboxes touch player health, actor clusters, boss state, etc.
A crate extraction needs ~15 inversions or pre-extracting half the lib first —
multi-session, not mechanical.

### dev — only ~1.3k cleanly movable (partial move-up)
Measured the real couplings (don't trust the module boundary):
- **`trace/` (~2.3k) STAYS lib** — `projectile` + `encounter` (sim) write
  `GameplayTraceEvent`/`GameplayTraceBuffer`. Sim-coupled.
- **`dev_tools.rs` (1.2k) STAYS lib** — `persistence::settings::model` reads the
  `DeveloperTools`/`Editable*`/`MovementProfile`/`PlayerBodyProfile` types AND calls
  `apply_movement_profile` / `apply_player_body_profile`. Presentation reads
  `DeveloperTools`. It's a read-only-state seam: the *types + apply fns* are
  lib-coupled; splitting the egui *systems* out is a carve, not a move.
- **`profiling.rs` (188) STAYS lib** — `audio::plugin` reads `phase_mark`.
- **`debug_overlay.rs` (995) + `fps_overlay.rs` (292) → app** — the F1 overlay + F3
  FPS counter have NO real lib consumer (persistence reads the `DeveloperTools.debug_overlay`
  *bool field* and only a *doc comment* mentions `FpsOverlayPlugin`). These two moved.

Guard: `architecture_boundaries_dev_overlays_live_in_app`.

**The bigger dev win (deferred):** carving `dev_tools.rs` into `dev_state` (types +
apply fns, lib) vs the egui inspector/sync *systems* (app), and slicing `trace` into
`model+buffer` (lib, sim writes) vs `detect+dump` (app, analysis). ~2.5–3k more to
app, but it's surgical file-splitting (like the audio runtime split), not a bulk move.

## Session 2 (2026-06-10, Opus) — actor unification + plugins paradigm

Jon's steers this session: (1) "components as plugins" — each module should be a
self-owning `Plugin`, not functions the app hand-wires; (2) "boundaries are mostly
shrunk but grow if they need to be right sized"; (3) **bosses ARE actors** (ADR 0016)
— so the brain/actor/boss tangle is not coupling-to-separate, it's that the
*right-sized unit* is actor + brain + generic boss-runtime as ONE component, with
only *named* boss content in `ambition_content`.

Done:
- **`BossEncounterPhase` → `brain::boss_pattern`** (commit 7fa4c2b3). Breaks the
  brain↔boss_encounter cycle (brain's last upward dep on boss_encounter, was 35
  refs); boss_encounter re-exports so consumers are untouched. First concrete step
  of the actor/boss unification.
- **`DevToolsPlugin`** (commit 63eb156d): the egui inspectors + FPS overlay, three
  scattered assembly calls → one `ambition_app::dev::DevToolsPlugin`.

Measured reality (the recurring finding): brain has **199 refs to `actor`** + upward
reaches into player (input frame), presentation (sheet lookup), mechanics
(`ActorPose`/`ActorFaction`), content (a test). The remaining sandbox core
(brain, actor, mechanics, features, world, presentation, boss_encounter) is a
**tightly-woven mesh** — the bisection already separated the easy axis
(content/machinery/app); what's left needs mesh-untangling, not bulk moves.

### Deferred backlog (real work, not mechanical — don't rush)
- **Complete `BrainPlugin`**: fold the scattered brain systems still registered in
  the app schedule (`player::tick_player_brains`, `brain::emit_brain_action_messages`,
  `emit_player_projectile_tick_messages`, `observe_brain_action_counter`) into
  `BrainPlugin` so brain self-owns registration. Risk: those carry explicit
  `.in_set(SandboxSet::…).after(…)` ordering; folding must preserve it or replay
  diverges. Do with the fixture gate per step.
- **Unified `ambition_actor` crate** (actor + brain + generic boss-runtime): the big
  shrink (~11k+). Blockers to invert first: actor→presentation sheet lookup,
  brain→player input types (move to `ambition_input`?), `ActorPose`/`ActorFaction`
  home, the actor→content test, and the named `BossAttackProfile` variants (data-key
  them like the sprite sheets/capabilities — see `dev/journals/code_smells.md`).
- **dev_tools state/systems carve**: split `dev_tools.rs` into lib `dev_state`
  (DeveloperTools + editable types + `apply_*`, read by persistence/presentation) vs
  app dev systems (`sync_*`); slice `trace` into lib model+buffer (sim writes) vs app
  detect+dump. ~2.5–3k more to app. Surgical, like the audio runtime split.

## Session 3 (2026-06-10, Fable 5) — `ambition_actor` EXTRACTED

The unified-actor-crate backlog item landed in one session because re-measurement
showed the survey's blockers had mostly dissolved (actor→presentation was
test-only; actor→content/state_machines were stale data). Commits:

- **f5208959** — prep inversions: `ActorPose`/`ActorFaction` → `actor::pose`
  (kit re-exports; `from_aabb` became parts-based `from_parts` so the vocabulary
  doesn't need the kit's body type); `PlayerSlot` → brain (`Brain::Player` embeds
  it); brain's `PlayerInputFrame` dep deleted (test-only wrapper; `BrainSnapshot`
  already carried `Option<ControlFrame>`); catalog↔sheet tests → presentation.
- **3e344d95** — **de-named `BossAttackProfile`**: GnuHandSlam→HandSlam,
  GnuAppleRain→DebrisRain, GradientLane→HazardColumn, OverfitVolley→MemorizedVolley,
  EyeBeam→LockOnBeam, MinimaTrap→PitTrap, SaddlePoint→RotatingCross,
  GradientCascade→MinionCascade, etc. CamelCase identifiers only — snake_case
  strings ("gnu_hand_slam", "eye_beam") are SPRITE-SHEET ROW KEYS and stay.
  RON authors the new names; profiles are now reusable behavior vocabulary.
- **beb0fe29** — **`crates/ambition_actor`** (~9.5k): actor control/AI/pose/faction
  + universal brain (+ `BossEncounterPhase`) + catalog schema/parser/resolver.
  Deps: engine_core + input crates only. Sandbox re-exports
  `pub use ambition_actor::{actor, brain}` → zero consumer churn.
  **Data/machinery split**: the game's roster moved to
  `ambition_sandbox::character_roster` (embeds the RON where the Python tools
  expect it; pre-loads the data-parameterized `CharacterCatalogPlugin`). Guard
  #23 scans the crate with ZERO exemptions — Jon challenged the first guard
  draft ("is the arch guard really correct?") and he was right: the catalog-dir
  exemption + the include_str escape hatch were masking an upward data dep.
  Splitting data from machinery made the exemptions unnecessary.

Test conservation: 187 (actor) + 992 (sandbox) = 1179, the exact pre-split count.
Replay bit-identical throughout. Sandbox lib is now ~101k (from 112k).

### Session 3b — boss-encounter core moved (8df30cdd); de-name verdict

- **`ambition_actor::boss_encounter`**: BossEncounterSpec schema + BossEncounterState
  phase machine + events moved (zero outward deps). The game's ten named spec ctors
  stay sandbox-side as the `BossSpecRoster` extension trait (`roster.rs`) — call
  sites unchanged modulo one trait import. NOT moved (measured): attack_geometry +
  behavior (couple to presentation's sprite-manifest types), damage/events/systems
  (sim glue), registry (→BossProfile→features), sprites (B3 pocket), ids.
- **Jon's de-name review verdict** (recorded in code_smells.md): the attack-profile
  rename is honest at the key/schedule/geometry/spec-parameter layers (any boss can
  author DebrisRain{interval,speed,damage} in RON today) but the EFFECTS consumers
  are still half-bespoke (apple art + some constants baked, gnu-named fns).
  Decision: accept + smell-log; no effect-composition framework until a second
  boss authors one of these specials.
- **BrainPlugin fold re-measured — doc-22's framing was WRONG**: the brain emitters
  are registered inside a `.chain()` INTERLEAVED with sandbox player systems
  (update_body_mode → tick_player_brains → sync_player_actor_poses → emitters) in
  `SandboxSet::PlayerInput`. The chain is the ordering contract; folding only the
  brain systems into the actor crate's BrainPlugin would break it. The right plugin
  boundary is the whole player-input pipeline (a sandbox-side plugin owning that
  chain). Replay-sensitive; do as its own focused slice.

## Honest takeaway for the next session
Remaining work, in rough value order:
- **Player-input pipeline plugin** (the corrected BrainPlugin item, see above):
  sandbox-side plugin owning the input-phase chain. Replay fixture gate per step.
- **dev carve** (state/systems split, ~2.5–3k) and the **B3 boss-asset map**
  (needs Jon's eyes — visual).
- **Special-effects parameterization finish** (smell-logged): lift baked apple
  art/constants into the RON specs when touched, or when a second boss needs one.
- The deep walls remain features (`EnemyConfig.archetype` knot), world (LDtk
  adapter), presentation, mechanics (~15 upward deps).

Pick the next target by the *measured outward-dep count*, not the line size or the
content-guard status — and re-measure before trusting this table; blockers rot.

## Session 4 (2026-06-10, Fable): portal presentation → its own crate + view cones

First real bite out of the **presentation → crate (B3)** row, taken from the
portal end rather than the boss-asset end (no visual-verification wall: the
moved visuals were behaviorally frozen and the new feature ships with pinned
math + UV tests).

- **`ambition_portal_presentation` extracted** (new workspace crate): the
  render-gated `portal/presentation.rs` (412 lines) left the sandbox whole.
  Seam contract mirrors `PortalPlugin`'s: crate-owned `PortalWorldFrame`
  (world-size sync; `world_size_to_bevy` factored into engine_core so the
  y-flip stays defined once), `PortalSceneBody` (host tags the player visual),
  `PortalGunArt` (host loads — asset paths are content), `PortalAimHint`
  (moved here; was never actually init'd anywhere — latent bug fixed, the
  held-gun aim hint now works). Sandbox keeps a ~100-line host adapter +
  facade re-export; `portal_render` now means `dep:ambition_portal_presentation`.
  Every visual is a separately flag-gated public system in
  `PortalPresentationSet` (extend-by-subtraction for other hosts).
- **Through-portal view cones** (new feature): `ambition_portal::view` proves
  the view map (body map ∘ entry-plane reflection) is ALWAYS a proper rotation
  — no mirror case exists — and `view_cone()` hands renderers the entry
  trapezoid + view-mapped source quad. The crate's renderer parks an
  axis-aligned capture `Camera2d` (render-to-texture) over the partner-side
  source rect and bakes the rotation into mesh UVs, so sim math is the single
  source of truth for what appears where. Cones live on the default layer, so
  capture cameras see other cones → **1-frame-lag infinite recursion** free,
  with no read-write hazard (cross-sampling only, by construction). Rigs are
  keyed + rebuilt only when a portal moves; captures re-render every frame.
- Guards: `architecture_boundaries_portal_presentation_crate_is_extracted`
  (deps ⊆ {engine_core, platformer_runtime, portal}; mechanic never names its
  renderer; no host refs), content-roster scan now covers the new crate, and
  the facade guard inverted (`portal/presentation.rs` must NOT exist).
- NOT yet runtime-verified visually (headless dev): cone orientation is pinned
  by `cone_uvs` unit tests, but the first in-game look is Jon's.

### Session 4 correction (same day): cones flipped to WINDOW semantics
Jon's first visual pass: cones were on the wrong side. The shipped model had
the view protruding into the room as a hologram at the entry; intended model
is a **window receding INTO the host surface** ("see through the portal a
little bit"). The window's display map is the plain BODY map (depth into the
entry wall = depth in front of the exit) — sight and transit share one map,
and the body map's mirror lands harmlessly in mesh UV space. Bonus: the
protruding design had an emergent artifact (the partner's cone sat inside
every capture rect at high alpha, so a cone mostly showed the viewer's OWN
side back — exactly the "wrong side" read); windows are in walls, captures
frame open rooms, so the artifact is structurally impossible. The
proper-rotation theorem stays in `ambition_portal::view` (it is the
camera-orientation tool for the projection model). Defaults retuned: depth
90 (≈ carve scale), spread 0.20, alpha 0.9. Still awaiting Jon's second look.

## Session 5 (2026-06-11, Fable) — the archetype knot dissolved (sim side)

Re-measured the features wall per the "blockers rot" rule and the
`EnemyConfig.archetype` knot had — like the actor-crate blockers before it —
mostly dissolved already: A2 had landed `EnemyTuning` + `CombatCapabilities` in
the kit, but the per-frame layer never switched over (caps was spawned and read
by NOBODY; the damage hook recomputed it from the enum each hit).

**Landed (`6dc440b9`), replay bit-identical:**
- `EnemyTuning` carries the full per-frame vocabulary; the named comparisons
  became authored data (`attack_cooldown_mult` 0.75/1.35 + `surface_walker` in
  enemy_archetypes.ron; `revives_in_place` keys off the existing
  respawn_in_place_seconds).
- `CombatCapabilities` joined the enemy cluster view; charge-crash + kill hook
  read the component.
- Every per-frame read (EnemyMut tick/aabb/visual_kind/begin_attack/body-contact,
  damage hook, presentation sandbag mapper) consumes spawn-projected data.
- Guard #25: `architecture_boundaries_enemy_sim_reads_data_not_the_archetype_enum`
  (actors.rs / damage.rs / presentation features.rs, production code).

**The enum is now spawn-vocabulary only** (scratch ctor `from_brain`,
spawn_actors, spawn_mounts composite fan-out, brain_builders, tests).

**Next milestone (measured, not yet done):** string-key the spawn seam — spawn
resolves a RON spec row by name (the `BRAIN_NAME_TO_ARCHETYPE` hop inverts to
name→row), brain_builders read the row, `ActorSpawnState`/`EnemyConfig` store
data not the enum — then the roster (enum + specs + RON + name table,
~enemies.rs) can leave the machinery lib for `ambition_content`, unblocking the
features→content shrink the survey table wants. Note: `ActorSpawnState`'s doc
claims runtime archetype morphing (dismount), but no production code assigns
`config.archetype` outside ctor/reset — verify the doc is stale (EnemyRuntime
era) before designing the baseline respec.

## Session 6 (2026-06-13, Fable) — the spawn seam, de-enumed

Finished the Session-5 milestone: the **persisted enemy component is now
archetype-free**, so the roster can leave the machinery lib next.

What the milestone actually required (bigger than flagged): the runtime brain
rebuilds — provoke-to-hostile (`aggression.rs`, hot path) AND mount dissolution
(`mount.rs`) — re-derived behavior from the enum on a LIVE entity. Decoupling
the per-frame layer wasn't enough; those two runtime paths had to reconstruct a
brain from data on the component too.

**Landed (one atomic change), replay bit-identical:**
- New generic kit vocabulary in `mechanics::combat`: `EnemyBrainTemplate` moved
  out of the roster (it was already template-generic) + a new `EnemyBrainSpec`
  (template + the three smash structural flags + the provoke MeleeBrute
  override) — the structural brain inputs, mirroring how `EnemyTuning` already
  carried the numeric ones.
- `EnemyConfig` DROPS `archetype`, GAINS `brain_spec`; `ActorSpawnState` DROPS
  `archetype` (the doc's "runtime morphing" claim was stale — confirmed: nothing
  morphs in place; the composite spawns two standalone entities and dismount
  swaps brain/action-set, never archetype). The enum now lives ONLY on the
  spawn-time `EnemyClusterSeed`, consumed before the entity exists.
- `brain_builders` provoke-reachable fns read `tuning` + `brain_spec`; the
  kit-derivation fns take an `EnemyArchetype` arg (resolved on the seed at
  spawn). Dismount rebuilds its action set from the rider's DURABLE stored
  `CombatKit` component + live held item — never the roster.
- `reset_to_spawn` no longer re-projects from the enum (tuning/brain_spec are
  immutable post-spawn — no morph — so the re-projection was a no-op).
- Guards: the per-frame guard drops `actors.rs` (now legitimately mixed:
  per-frame tick helpers + spawn-time NPC→enemy conversion) and a new
  structural guard `architecture_boundaries_enemy_config_is_archetype_free`
  slices the `EnemyConfig` + `EnemyMut` struct bodies and forbids the enum —
  a stronger invariant than the file-level scan it replaces.

**Next (now unblocked):** lift the roster (`EnemyArchetype` enum + specs + RON +
`BRAIN_NAME_TO_ARCHETYPE`, ~`enemies.rs`) into `ambition_content`. The seam is a
single spawn hop (`EnemyArchetype::from_brain` → seed projection); nothing
persisted or per-frame names it anymore. The one named reference left in the
lib's runtime path is dismount's `EnemyArchetype::PirateRaider.melee_spec()`
fallback — a pirate-mechanic constant, not a stored-enum read; fold it into the
content move.

## Session 7 (2026-06-13, Fable) — enemy behaviors fully data-driven (roster-move prep)

Jon chose "go straight to the content-crate move" (lift the enemy roster out of
the machinery lib into `ambition_content`). The move requires the lib spawn path
to stop naming `EnemyArchetype`; this session removed the last *behavioral*
coupling.

**Landed (`08d1afe0`), replay bit-identical:** the four behaviors still hardcoded
as `match self { … }` arms on the enum — `attacks_player`,
`body_contact_damage`, respawn cadence, and the smash/provoke brain flags —
became authored `EnemyArchetypeSpec` fields (RON annotates only the exceptional
rows; defaults cover the common case). The projection logic moved onto
`impl EnemyArchetypeSpec`; the enum methods delegate to `self.spec().*`. A new
parity test re-encodes the OLD identity formulas as the oracle and asserts every
archetype's RON row reproduces them (guards the exotic rows replay never runs).

**The enum is now a pure spawn-time RESOLUTION handle** — every projection is
spec-driven; the only enum logic left is `from_brain` (brain-key string → enum)
and the registry key. So `EnemyArchetype` could be replaced by the spec
everywhere with no behavior change.

**Remaining to actually move the roster (precise plan):**
1. **String-key the spawn path (1B):** add `spec_for_brain(&EnemyBrain) ->
   EnemyArchetypeSpec`; carry `EnemyArchetypeSpec` (not the enum) on
   `EnemyClusterSeed`; convert the brain_builders kit fns
   (`enemy_combat_kit_for_archetype` / `enemy_default_action_set` /
   `held_item_for_archetype` / `mounted_rider_*`) to take `&EnemyArchetypeSpec`;
   drive the composite fan-out (`spawn_mounts`) + `is_composite_spawn` off
   `spec.composite_visual` instead of matching `PirateOnShark|PirateHeavyOnShark`.
   After this the enum is vestigial (used only inside `from_brain`).
   ~8 files (features/ecs/spawn*, brain_builders, mount, actors, visual mappers
   in enemies.rs). Mechanical; gate replay.
2. **Invert spec resolution to a Resource (2a):** the spec registry is a
   `LazyLock` over embedded RON today, read by the free fn `archetype_spec()`
   (callable from non-system contexts). Turn it into a lib-defined
   `Resource<EnemyRoster>(HashMap<String, EnemyArchetypeSpec>)` populated at
   startup; resolve specs from the resource in spawn systems (the
   CharacterCatalog data-parameterized pattern). The non-system callers
   (`EnemyClusterSeed::new`, any `BossBehaviorProfile` ctor) take the resolved
   spec as a param.
3. **Relocate (2b):** move the roster (`EnemyArchetype` enum +
   `BRAIN_NAME_TO_ARCHETYPE` + the RON + the registry-population) and the named
   composite fan-out into `ambition_content`; content's plugin inserts the
   `EnemyRoster` resource at startup. The generic `EnemyArchetypeSpec` schema +
   the generic spawn machinery stay in the lib. Fold dismount's lone
   `EnemyArchetype::PirateRaider.melee_spec()` constant into the content move.
   Add an architecture_boundaries guard: the lib never names `EnemyArchetype`.

Gates every checkpoint: ambition_sandbox lib + architecture_boundaries +
replay_fixture_regression (zero-divergence) + scripted_gameplay +
build --features visible.

### Session 7b (2026-06-13, Fable) — spawn path string-keyed; enum vestigial (1B done)

Landed `f3a637b3` (replay bit-identical). The spawn pipeline now resolves and
carries `EnemyArchetypeSpec` by string key (`spec_for_brain(&EnemyBrain)`); the
named composite fan-out is driven by the authored `composite_visual` row. In
PRODUCTION the `EnemyArchetype` enum survives only inside the roster's own
resolver (`from_brain` + `BRAIN_NAME_TO_ARCHETYPE`) — every spawn-path file
(`spawn_actors`/`spawn_mounts`/`brain_builders`/`mount`/`actors`) is enum-free.
The dead projection delegators were deleted (they duplicated `EnemyArchetypeSpec`);
`is_composite_spawn` (named match) → `spec.is_composite()`. Also fixed an
orphaned `#[cfg(test)]` that was gating a production `SpawnProjectile` import —
the lib-test build masked it; the app gate caught it (reproduce-under-real-cfg).

**Remaining = Checkpoint 2 (the actual relocation):**
- **2a — invert spec resolution to a Resource.** Today the registry is a
  `LazyLock<HashMap<String, EnemyArchetypeSpec>>` over embedded RON, read by the
  free fn `archetype_spec()`/`spec_for_brain()` (callable from non-system
  contexts — `EnemyClusterSeed::new`, boss ctors). DESIGN DECISION: to let
  content own the RON, the spec data must come from a Bevy `Resource`
  (`EnemyRoster`) the content plugin inserts at startup. The non-system callers
  (`EnemyClusterSeed::new`) then need the registry passed in — i.e. spawn
  systems resolve `&EnemyArchetypeSpec` from `Res<EnemyRoster>` and hand it to
  `new()`. This ripples through the spawn entry points (room loader → `spawn_enemy`
  → `EnemyClusterSeed::new`). Keep the lib's generic `EnemyArchetypeSpec` schema;
  move only the DATA.
- **2b — relocate.** Move `EnemyArchetype` + `BRAIN_NAME_TO_ARCHETYPE` + the RON
  + registry-population into `ambition_content`; its plugin inserts the
  `EnemyRoster` resource. Fold dismount's lone `PirateRaider.spec().melee_spec()`
  constant into content (or replace with a generic "dismount fallback melee" spec
  field). Add an architecture_boundaries guard: the machinery lib never names
  `EnemyArchetype`.

### Session 7c (2026-06-13, Fable) — resolution inverted to an installable holder (2a done)

Landed `2c65d963` (replay bit-identical). Production spec resolution now goes
through `EnemyRoster { by_brain, fallback }` — `spec_for_brain` is a pure
string lookup, so the `EnemyArchetype` enum is gone from the resolution path
(only the embedded-default builder + tests still name it). `install_enemy_roster`
(OnceLock override) is the seam content plugs into; the lib ships an embedded
default (bundled RON + `BRAIN_NAME_TO_ARCHETYPE`) so it resolves standalone.
Chosen as an installable global, NOT a Bevy `Resource`, because resolution is
read from many non-system contexts (constructors, presentation sprite-binding,
asset resolution) — threading `Res` would be a pervasive ripple (documented at
the type).

**Checkpoint 2b — the relocation (precise plan + the one real decision):**

The blocker to fully evicting the enum: the lib's OWN tests + headless bin need
a roster to spawn enemies. So the design decision is *where the embedded default
lives*:
- **Recommended:** re-key `enemy_archetypes.ron` by BRAIN KEY (`"medium_striker"`
  not `"MediumStriker"`; add a reserved `"combatant"` fallback row). Then
  `EnemyRoster::from_ron(&str)` parses a brain-keyed map directly — NO enum, NO
  `BRAIN_NAME_TO_ARCHETYPE` needed to build it. The embedded default becomes
  `from_ron(include_str!(bundled))`, fully enum-free. This makes the enum 100%
  test/authoring-only in the lib, ready to move.
- Then **relocate** to `ambition_content`: the enum (+ `COMBAT_ALL`,
  `from_brain`, `archetype_data_key`), the brain-name table, the RON, and the
  roster tests (conversion_tests invariants, parity, capability). Content's
  plugin calls `install_enemy_roster(EnemyRoster::from_ron(MY_RON))` at startup.
- The lib KEEPS: `EnemyArchetypeSpec` (bump to `pub`) + its projection methods,
  the `EnemyRoster` holder + `from_ron` parser + install seam, and the generic
  spawn machinery. For lib standalone tests, either (a) keep a tiny brain-keyed
  test RON in the lib, or (b) move the enemy-spawn lib tests to content/app
  where the roster is installed. (a) is simpler and keeps lib tests hermetic.
- Add an `architecture_boundaries` guard: the machinery lib never names
  `EnemyArchetype`.

Scope: cross-crate move + RON re-key (replay-sensitive — the brain-key→spec
mapping must be preserved exactly) + ~40 enum-test-site relocations. A focused
session. Gate replay every step.

### Session 7d (2026-06-13, Fable) — lib production fully enum-free (2b.1 + last logic ref)

Landed `9e2b3d8c` + `43b5861a` (replay bit-identical):
- **2b.1**: re-keyed `enemy_archetypes.ron` by spawn brain key; `EnemyRoster`
  gained `from_map`/`from_ron`; the embedded default parses the brain-keyed
  registry with NO `EnemyArchetype` in the build. `archetype_data_key` returns
  brain keys so the (test-only) enum→spec path still resolves.
- Dismount's last enum logic ref → `spec_for_brain(Custom("pirate_raider"))`,
  which made the enum→spec chain (`archetype_spec` / `archetype_data_key` /
  `EnemyArchetype::spec()`) production-dead → now `#[cfg(test)]`-gated.

**State: the machinery lib's production code (logic + resolution + embedded
default) names `EnemyArchetype` NOWHERE.** The enum is a `#[cfg(test)]`/`pub`
typed handle (definition + re-export + ~50 test sites). The architectural
decoupling is complete; what remains is physical relocation.

**Checkpoint 2b.3 — the relocation (fresh-session task; two real gotchas):**
1. **Install ordering (runtime correctness!).** Content's plugin must
   `install_enemy_roster(EnemyRoster::from_ron(CONTENT_RON))` BEFORE the first
   room spawns an enemy. `install_enemy_roster` is first-write-wins, so if the
   lib's embedded default is touched first (any spawn / `spec_for_brain`) the
   override silently loses. Either (a) install in a `PreStartup`/early-startup
   system ordered before room load, or (b) make the override settable-once but
   checked lazily so first *install* wins regardless of first *read*. Verify
   with a test that installs a divergent roster and asserts a spawn uses it.
2. **Lib test fixture.** Moving the enum + RON to content breaks the lib's own
   enemy-spawn tests (conversion_tests 28, spawn.rs 21, capability/parity/data
   tests). Plan: move the roster-DATA tests (HP/aggro invariants, capability
   parity) to content alongside the enum; for the lib MACHINERY tests
   (spawn-attaches-brain, action-set derivation, dismount), add a tiny
   brain-keyed test-fixture RON in the lib and switch them to brain-key strings
   via `spec_for_brain` (drop the enum). Bump `EnemyArchetypeSpec` / `EnemyRoster`
   / `install_enemy_roster` to `pub`.
3. Add an `architecture_boundaries` guard: the machinery lib's production code
   never names `EnemyArchetype` (exempt the relocated-away definition).
