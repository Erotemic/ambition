# The decomposition playbook — killing the monolith, carve by carve

**Authored by fable, 2026-07-05; role-anchored 2026-07-06.** THE
highest-priority engineering track (Jon, binding): `ambition_gameplay_core`
(~95k LOC) and the fat app must decompose into the crate set of
[`architecture.md`](architecture.md) — referred to below BY ROLE (e.g.
*[the sim heart]*, *[the space IR]*) — so that (a) small agents can
navigate and modify any one domain safely, (b) content/demos plug in
without touching core, (c) hot-edit rebuilds shrink. This doc is the
ORDERED playbook with per-carve task cards and exit criteria; live
statuses live in [`../tracks.md`](../tracks.md). The specs here are ported
from the archived review adjudications and remain binding — an executor
needs no archived doc to proceed.

**Anchor style (evergreen):** cards cite `path` + SYMBOL (function/type
name), never line numbers. If a named symbol has moved or been renamed,
`rg` for it; if it's gone, that's drift — update the card in the same
commit (living-plan discipline), don't guess.

**Fable/opus handoff markers (convention, grep-stable).** Because fable's
availability is ending (Jon, 2026-07-06), every unresolved design decision
is tagged so a future opus agent knows exactly where it may NOT improvise:
- **`QUESTION FOR FABLE [tag]`** — a design/doctrine decision fable owns.
  **(As of 2026-07-06 night ZERO live markers remain — every one was
  ruled before the window closed. The convention stays documented for
  reading old commits; a NEW design ambiguity follows the post-fable
  decision-brief protocol in tracks.md instead.)**
- **`OPUS-SAFE`** — the doctrine is settled; the remaining work is
  mechanical and opus executes it directly (no design freedom).
When a `QUESTION FOR FABLE` is resolved, replace it with the ruling + flip
the dependent work to `OPUS-SAFE` in the same commit.

**Method rules (all carves):**

- **Measure OUTWARD deps first.** "Names no content" ≠ "extractable"; a
  module with dozens of inbound mechanic deps stays until inversions land.
- **The D2 template:** kill cycles/misplacements INSIDE the crate first
  (compiling, committable steps), then ONE atomic move of the module to
  its crate, then repoint every consumer. Never a lasting facade; delete
  re-export shims in the same arc.
- **Compile-parity gates:** after each carve, `cargo build -p
  ambition_app --features rl_sim` + the suite trio (gameplay_core lib,
  content, app rl_sim) + the architecture-boundary tests. Schedule shape
  is pinned by the rl_sim headless app tests (they caught the G3 cycle).
- **Feature discipline:** `ambition_runtime` forwards
  `headless`+`input`+`portal_ldtk`; new crates declare features
  explicitly; never rely on unification accidents.
- **Record compile-time before/after** per carve (the point is partly
  rebuild speed — keep the receipts, `cargo build -p <crate> --timings`).

## Anti-god-structure rules (BINDING on every executor)

The failure mode this playbook exists to prevent is re-centralization —
an agent "simplifying" by putting things in one place. These are hard
rules; violating them is wrong even when it compiles and reads cleaner
to you:

1. **No `utils`/`common`/`shared`/`prelude`-dump crates, ever.** A type
   with no clear owner means the classification is unfinished — finish
   it (the E2 classification rule generalizes: vocabulary moves DOWN to
   the crate that OWNS the domain; facts invert to parameters).
2. **Every moved module keeps/ships its OWN `Plugin`** registering its
   own systems/messages/resources. The runtime GROUP composes plugins;
   it never absorbs their registrations inline. If a carve leaves a
   crate without a plugin (pure vocabulary), that's fine — but never the
   reverse (a plugin registering another domain's systems).
3. **The `features/` hub facade DIES; it does not migrate.** No new
   re-export hub may be created in `ambition_actors`, the runtime, or
   anywhere else. Consumers import from the owning crate, explicitly
   (the explicit-imports rule). A "convenience" hub is the monolith's
   ghost.
4. **One-way doors:** a lower tier may NEVER import a higher one, and
   sibling domain crates may not import each other except along the
   arrows architecture.md draws (combat ← actors, persistence ← menu,
   …). When you want a sideways import, you have found either (a) a
   vocabulary type that belongs a tier down, or (b) a fact that should
   be a parameter/message. There is no (c).
5. **Resources are owned.** A resource is defined + initialized in
   exactly one crate (its plugin); other crates read it via system
   params. Cross-crate `init_resource` of another domain's type is a
   review flag.
6. **When splitting, split by AUTHORITY (who mutates), not by theme.**
   "All the boss stuff together" is a theme; "the systems that mutate
   BossPhaseState" is an authority. Themes produce god crates.

---

## THE LEDGER — every module, measured, with its destination

*(Measured 2026-07-05 from the tree. This is the ground truth the carves
execute against; an executor whose module doesn't match this table has
found drift — update the table in the same commit. LOC ≈ `wc -l`.)*

### `ambition_gameplay_core` (~95k) disposition

| Module | LOC | Destination crate | Carve | Notes |
|---|---:|---|---|---|
| `features/ecs/` core (spawn_actors 1241, perception 856, actor_clusters 723, view_index 609, aggression 488, brain_effects 457, anim_helpers 446, brain_builders 433, interact 300, + `actors/` 2705, `spawn/` 468) | ~9.7k | **`ambition_actors`** (Tier 3) | E7 rename | the actor sim heart: spawn/tick/perceive/act |
| `features/ecs/damage/` | 1914 | `ambition_actors` | E7 | victim-side resolution stays with the bodies it mutates; the HIT VOCABULARY moves with combat (E2 classification rule) |
| `features/ecs/mount/` | 1969 | `ambition_actors` | E7 | mounts are sim (ADR 0020) |
| `features/ecs/bosses/` | 1701 | `ambition_actors` (behavior residue → `ambition_characters` per E6d) | E6→E7 | shrinks as E6 folds the brain-tick |
| `features/ecs/encounter_rewards.rs` | 365 | `ambition_encounter` | E-enc | with encounter/ below |
| `features/enemies/` | 2188 | `ambition_actors` (schema) — archetype DATA already content | E7 | respawn-policy slice edits here first |
| `features/bosses.rs`, `npcs.rs`, `banter.rs` | ~1.3k | `ambition_actors` | E7 | |
| `combat/` (moveset 2292, damage 944, targeting 892, attack 834, hitbox/, on_hit 430, events 432, components/, pickups 340, world_overlay 360, boss_clusters 444) | 11.5k | **`ambition_combat`** | **E2** | world_overlay → `ambition_world` (it's geometry composition); boss_clusters dissolves with E6 |
| `projectile/` + `enemy_projectile/` | 4412 | **`ambition_projectiles`** | E2 | visual-kind content names leave in E3 |
| `world/` | 10933 | **`ambition_world`** (IR) + **`ambition_ldtk_map`** (backend) | **W1–W4** | the two-crate cut; converter registry is the keystone |
| `boss_encounter/` (behavior, registry, sprites, attack_geometry, encounter_script, rewards) | 6750 | behavior/registry → `ambition_characters`; sprites/attack_geometry → `ambition_sprite_sheet`; encounter_script/rewards → `ambition_encounter` | E6 + E3 + E-enc | the three-way split the plan always intended |
| `player/` (body_integration, bundles, starting_character, trail, affordances) | 6511 | `ambition_actors` | E7 | the home body is A BODY; no player crate — that would re-fork the unification |
| `persistence/` + `host/` + `quest/` | 5173 | **`ambition_persistence`** | **E1a** | owns stored-shape only; settings IR stays for E1e |
| `character_sprites/` | 4335 | **`ambition_sprite_sheet`** | **E3** | + the asset-root flip |
| `abilities/` + `ability_cooldown.rs` | 4211 | `ambition_actors`; **D-B carve candidate `ambition_abilities`** | E7→D-B | traversal kit reads controlled-subject + kinematics; carve iff outward-dep measurement is clean |
| `assets/` | 3324 | `ambition_asset_manager` | E-assets | mechanical absorb |
| `menu/` | 3189 | **`ambition_menu`** | **E1e** | + app/menu (below); LAST of E1 |
| `dev/` | 2975 | **`ambition_dev_tools`** | E1d | + app/dev |
| `items/` + `inventory_ui/` | 2689 | **`ambition_items`** | E8 | |
| `encounter/` | 2504 | `ambition_encounter` | E-enc | wave/lockdown kit |
| `dialog/` | 2217 | **`ambition_dialog`** (runtime) | E1c | game bindings stay sim-side |
| `time/` | 1431 | stays (measured: depends on player/combat/features); `camera_ease` rides E4 | E8 note | |
| `audio/` + `music/` | 1791 | **`ambition_audio`** | E1b | |
| `session/` | 1245 | `ambition_actors` | E7 | lifecycle of the sim |
| `body_mode/` | 807 | `ambition_actors` | E7 | mode→sprite-state seam lands in E3 but the MODE is sim |
| `portal/` (glue) | 711 | `ambition_actors` glue or `ambition_portal` adapter | E7 | measure at rename time |
| `schedule/` | 599 | `ambition_runtime` (the set vocabulary) | E5-finish | `configure_sandbox_sets` moves with it |
| `gravity/` | 252 | `ambition_actors` | E7 | |
| `camera_snapshot.rs` | 459 | **`ambition_sim_view`** | **E4** | + per-body velocity (AJ14 Tier-0) |
| `character_roster.rs` | 572 | `ambition_actors` (install seam) | E7 | |
| `shrine.rs`, `cutscene.rs`, `actor.rs`, misc | ~550 | `ambition_actors` / `ambition_cutscene` | E7 | |
| `platformer_runtime/` | 72 | `ambition_runtime` | E5 | already thin |

### `ambition_app` (~24.6k) disposition

| Module | LOC | Destination | Carve |
|---|---:|---|---|
| `menu/` | 10046 | `ambition_menu` | E1e (the misplaced elephant) |
| `app/` (boot/plugins/resources/cli) | 4879 | split: engine-generic sim wiring → `ambition_runtime`; windowed host wiring → **`ambition_host`**; content installs + asset roots + window + Ambition choices stay | E5-finish steps 1–5 |
| `dev/` | 2714 | `ambition_dev_tools` | E1d |
| `rl_sim/` | 1612 | stays app-side, thinned onto `add_headless_foundation`; the generic stepping harness is a D-B candidate for `ambition_runtime` | E5-finish 2 |
| `bin/`, `host/` | 1760 | stays (the thin shell IS the point) | — |

**Post-carve `ambition_actors` ≈ 33k** (features-core + player + abilities
+ session + body_mode + small glue) — ONE concern: the unified actor
simulation. D-B then re-measures the `ambition_abilities` candidate
(~4.5k) and stops there unless the numbers argue otherwise (U1).

### The projected end-state sizes (measured 2026-07-06 night, 101.7k total)

What the ledger above adds up to, so "how much does the monolith shrink"
has a standing numeric answer (re-measure when the tree drifts):

| Destination (carve) | From gameplay_core modules | ~LOC out |
|---|---|---:|
| `ambition_combat` (E2) | `combat/` | 12.8k |
| `ambition_world` + `ambition_ldtk_map` (W3) | `world/` | 10.9k |
| 3-way E6/E3/E-enc split | `boss_encounter/` | 6.8k |
| `ambition_persistence` (E1a) | `persistence/ quest/ host/` | 5.2k |
| `ambition_projectiles` (E2) | `projectile/ enemy_projectile/` | 4.4k |
| `ambition_sprite_sheet` (E3) | `character_sprites/` | 4.3k |
| `ambition_asset_manager` (E-assets) | `assets/` | 3.3k |
| `ambition_menu` (E1e) | `menu/` | 3.2k |
| `ambition_dev_tools` (E1d) | `dev/` | 3.0k |
| `ambition_items` (E8) | `items/ inventory_ui/` | 2.7k |
| `ambition_encounter` (E-enc) | `encounter/` + rewards | 2.9k |
| `ambition_dialog` (E1c) | `dialog/` | 2.3k |
| `ambition_audio` (E1b) | `audio/ music/` | 1.8k |
| `ambition_runtime` (E5 tail) | `schedule/ platformer_runtime/` | 0.7k |
| **total leaving** | | **≈ 64k (63%)** |

**Residual [the sim heart] ≈ 35k** (`features/` 20.6k + `player/` 6.6k +
`abilities/` 4.2k + `time/` 1.4k + `session/` 1.3k + `body_mode/` +
portal glue + gravity + roster/shrine/cutscene misc), dropping to
**≈ 31k** if D-B's abilities re-measurement carves `ambition_abilities`
(~4.2k) — consistent with the ledger's ≈33k estimate. That residual is
the DELIBERATE floor, not unfinished work: splitting spawn/tick/perceive/
damage-routing apart would re-fork the actor unification (U1). Below the
crate line, navigability is won by the D-B internal standard (every
module ≤ ~1.5k lines, one concern, `MODULES.md`), not by more crates.
End state: the largest engine crate is [the sim heart] at ~31–35k
(≈ 2× engine_core/characters); nothing else exceeds ~13k.

**Efficiency (why the split costs the game nothing):** crate boundaries
are COMPILE-TIME structure — the same systems run in the same schedule
(E5's carve was byte-parity-gated, the precedent). Rust inlines generics
across crates and `#[inline]` covers the hot small fns; the kernels
already live in engine_core; thin-LTO on release builds is the lever if
a boundary ever shows in a profile (none expected). The costs that DO
exist are paid deliberately: the E4 read-model copies view facts once
per tick (bought: netcode/RL/render decoupling — Q32), and the win the
split exists for is INCREMENTAL COMPILE (touch a combat file → rebuild
~13k + dependents, not 101.7k + everything; the per-carve `--timings`
receipts are the standing measurement).

### Why these pieces are THE pieces (the elegance argument)

The crate boundaries follow the four real fault lines in the domain, not
line counts: (1) **vocabulary vs. simulation** — schemas/registries/
formats (entity_catalog, sprite_sheet, characters) sit below the systems
that step them (actors, combat), so content and tools can depend on
vocabulary without dragging the sim; (2) **sim vs. space** — the world/IR
(`ambition_world`) is authored INPUT to the sim, never a peer
(backend-agnostic by construction, which is what makes Tiled/Godot
importers additive); (3) **sim vs. observation** — `ambition_sim_view` is
the one-way read-model boundary (render, netcode confirmation, RL
observation, and the slower-light shaders are all THE SAME KIND of
consumer); (4) **engine vs. host vs. content** — `ambition_runtime`
(headless sim assembly) / `ambition_host` (windowed wiring) /
content crates (named worlds+rosters+rules). Every demo and the game
compose from exactly these five faces, which is why the same
decomposition serves all of them.

### The demo/game → crate support matrix (the proof of sufficiency)

| Consumer | Exercises beyond the shared core |
|---|---|
| Sanic | momentum kernel (engine_core::surface), `ambition_world` chains channel, mode-scope seam |
| Super Mary-O | `ambition_items` equipment policies, camera policy knobs, cutscene kit |
| Super Smash Siblings | `ambition_combat` CM stack, N1 slot routing (`ambition_host`), fighter brain (`ambition_characters`), `ambition_sim_view` damage-meter read |
| Hollow Lite | boss pipeline (characters + encounter + combat), `ambition_persistence` (benches), respawn policy (actors) |
| Ambition itself | ALL of the above + portals, dialog, menu, audio, falling-sand content plugin — and hosts each demo via mode scopes |

Shared core in every column: runtime + host + actors + combat + world +
sim_view + characters + entity_catalog + input. If a demo needs a crate
edit outside its column's expectation, that's the oracle firing.

---

## Phase D-A — finish the engine face (the current arc)

**Every carve below is a TASK CARD: preconditions → ordered steps (each
step compiles and is committable) → exit checks. File anchors are from
the 2026-07-05 inventory; if the code has drifted, update the card in
the same commit that exploits the drift. The demos, netcode, combat,
brain, and boss tracks are DOWNSTREAM of this phase — an agent picking
up work executes D-A cards first unless tracks.md's queue says
otherwise.**

### E5-finish — [the sim assembly] completes (THE DEMO GATE) — [opus, fable-specced]

Precondition: none — the crate exists (`crates/ambition_runtime`; the app
consumes `PlatformerEnginePlugins` inside `add_simulation_plugins`,
`ambition_app/src/app/plugins.rs`).

- **Step 1 — sets + resources into the group.** Move the
  `configure_sandbox_sets(app)` call and the adjacent sim `init_resource`
  block (`ShrineActivationPulse`, `SlotInteractionState`,
  `StartingCharacter`) from `add_simulation_plugins` into
  `PlatformerEnginePlugins`' FIRST plugin (a small `SandboxSetsPlugin`
  inside [the sim assembly]), so the set vocabulary exists before any
  `in_set(...)` registration. Keep `init_resource` semantics (hosts
  override by `insert_resource` BEFORE `add_plugins` — the CLI's
  `insert_starting_character_override` relies on it; add a test:
  insert-then-add preserves the override).
- **Step 2 — `CombatSchedulePlugin` into the group.** It imports only
  gameplay_core + `ambition_vfx` (`app/combat_schedule.rs`) — add
  `ambition_vfx` to [the sim assembly]'s deps. Its
  `CombatSet::ContentSpecials`/`ContentFlavor` label slots stay
  registered-but-empty; content attaches app-side (that file's
  content-slot guard test must stay green).
- **Step 3 — `add_headless_foundation(app)`** in [the sim assembly]:
  the MinimalPlugins+AssetPlugin+ImagePlugin+TransformPlugin+
  StatesPlugin+`init_state::<GameMode>` block copy-pasted three times
  (`app/headless.rs`, its test module, `rl_sim/runtime.rs`). Replace all
  three; the visible path shares only the `init_state` piece via a
  second helper `init_engine_states(app)`.
- **Step 4 — de-weave content.** Move the cut-rope systems out of the
  engine chains: `emit_cut_rope_room_replay_after_dialogue_closes` +
  `apply_cut_rope_room_replay_request_system` (registered inside
  `register_player_input_systems`) and
  `reset_cut_rope_boss_arena_on_room_reset` (inside
  `register_room_transition_systems`) move to `AmbitionContentPlugin`,
  attached via labeled sets (`ContentRoomResetSet` exists; add a
  `ContentDialogueFollowupSet` anchored after the dialogue-close
  system). The app chains become pure-engine tuples.
- **Step 5 — ✅ EXECUTED (fable, 2026-07-06 night) with ONE amendment to
  this card.** The card said "move the five register fns to
  `ambition_host`"; execution found that is WRONG for four of them —
  `add_simulation_plugins` (which registers them) is added by BOTH the
  visible binary AND headless/RL (`headless.rs`, `rl_sim/runtime.rs`,
  every portal/gravity parity suite), and the scaffold doctrine
  "a headless entry point adds ONLY the engine group" wins. So the
  shared per-frame SIM wiring went to **`ambition_runtime`** (the engine
  group) as per-domain plugins — `PlayerSchedulePlugin` (time-control →
  input → controlled-subject → brains → possession → hit events →
  presentation write-back, + the brain-emitter block),
  `RoomTransitionSchedulePlugin` (detect + feature reset + the content
  slot), `PortalSchedulePlugin` (PortalPlugin + the three ordering
  landmines, feature `portal`), `ProgressionSchedulePlugin` (moved
  file) — and **`ambition_host`** received only the genuinely WINDOWED
  wiring: `HostInputBindingsPlugin` (leafwing map + device→ControlFrame
  bridge, feature `input`; startup attach rides the new
  `SimulationSetupSet` label instead of naming the app's setup system)
  and `HostCameraPlugin` (viewport publish → shake → `camera_follow`,
  + portal camera continuity under `portal_render`). The app-local
  residue pins itself into two documented ordering SLOTS the engine
  chains leave open (reset/replay pair in PlayerInput; home-reset/
  presentation pair in PlayerSimulation; the room-transition APPLY
  composer in RoomTransition) — see `register_app_local_sim_systems`
  (`ambition_app/src/app/plugins.rs`) + the runtime plugins' module
  docs. Parity: full app rl_sim suite (portal/gravity/continuity/
  replay-fixture) green, zero behavior change.

  #### ✅ READINESS BRIEF (opus 2026-07-06) — HISTORICAL; superseded by the executed step-5 amendment above
  *(The brief's "MOVES to `ambition_host`" destination was amended at
  execution: the shared sim wiring landed in `ambition_runtime` because
  headless/RL registers it too; only leafwing input + the camera cluster
  landed in `ambition_host`. The system-by-system MOVES/STAY
  classification below was correct and was executed as written.)*

  **E5 step 5 is NOT gated on the E1d/E1e crate mints.** `ambition_host`
  MAY dep `gameplay_core`, so any host-set system living in gameplay_core
  is a clean lift — no prior split needed. Investigated every system in
  the five movable `register_*` fns (`ambition_app/src/app/plugins.rs`)
  and classified it. The carve is a well-bounded lift; the ONLY careful
  part is the portal-schedule ordering (fable's graded work).

  **MOVES to `ambition_host` (all resolve to gameplay_core/render/input/
  runtime, which host may dep):**
  - `register_player_input_systems` — entire chain: `time::time_control::*`,
    `refresh_world_time`, `mirror_sim_dt_into_runtime`,
    `dev::sync_live_player_dev_edits_system` (gameplay_core::dev ✓),
    `input_timer`/`interaction_input`, the controlled-subject chain,
    `tick_player_brains`, `body_mode::update_body_mode`,
    `sync_player_actor_poses`, the `ambition_characters::brain::*`
    emitters, `apply_room_replay_request_system` (generic, de-woven step 4).
  - `add_input_plugins` — the menu/gameplay input systems are ALL
    `gameplay_core::schedule::input_systems`
    (`populate_menu_control_frame_from_actions`,
    `populate_control_frame_from_actions`,
    `apply_menu_frame_to_cutscene_request`,
    `toggle_player_trail_emission_from_actions`) + `ambition_input::*` +
    `dialog::dialog_pointer_input` (gameplay_core::dialog ✓). **The
    "menu entanglement" in the old accounting is a NON-ISSUE** — menu
    INPUT is gameplay_core; the app menu crate (E1e) is irrelevant here.
  - `register_room_transition_systems` — `rooms::detect_room_transition_system`
    + `features::reset_ecs_room_features` (gameplay_core) MOVE; the two
    APP wrappers below STAY.

  **STAY app-side (the app registers these alongside the host group; host
  leaves the slots — the card's "app-local pieces stay" made concrete):**
  - `register_player_simulation_systems`: `crate::app::player_clone::*`
    (already its own module), `apply_home_reset_policy`
    (`app/player_tick.rs`), `sync_player_presentation` (`app/phases.rs`),
    `apply_player_reset_input_system` (`app/sim_systems.rs`).
  - `register_room_transition_systems`: `apply_room_transition_system` +
    `ensure_requested_room_parallax_system` (both `app/world_flow/
    room_flow.rs` — they call `load_room`/render spawns).
  - `add_input_plugins`: `sync_preset_input_map` + `handle_debug_hotkeys`
    (`app/dev_runtime.rs` — the ONLY genuinely app-local dev systems in the
    whole set; two of them, not a blocker).

  **THE ONE CAREFUL PART (fable-graded):** `wire_portal_schedule` (behind
  the `portal` feature) pins three named-system ordering landmines that
  must survive the move verbatim: **Carves** `.after(physics::
  collect_gravity_zones).before(CoreSimulation)`; **InputWarp** `.after(
  player::interaction_input_system).before(player::
  sync_local_player_input_frame)`; **RoomReset** reset-time portal cleanup
  in the room-transition phase. These reference gameplay_core systems (host
  may dep), so the pins compile after the move — but the ORDER is the feel/
  correctness contract; verify against the portal-continuity + gravity-zone
  suites after the lift.

  Net: mint `ambition_host`, move the MOVES set as per-domain plugins
  (anti-god rule 2), keep the STAY set registered app-side, preserve the
  three portal pins. No E1d/E1e prerequisite. Exit per below.

  **(Historical scaffold note, superseded by execution:** the empty
  `HostSeamPlugin` scaffold this brief describes was replaced by the real
  `HostInputBindingsPlugin`/`HostCameraPlugin` at step 5; the no-content
  boundary test carried over.)
  **PARITY HARNESS ALREADY EXISTS — port boldly.** The portal ordering is
  covered end-to-end by `ambition_app/tests/{portal_bridge_reachability,
  portal_translation_camera_continuity, projectile_portal_transit,
  held_projectile_portal_transit, portal_floor_bounce_no_fallthrough,
  portal_reset_preserves_authored, portal_lab_usable}` and the gravity
  carves by `{gravity_room_reachability, gravity_symmetry,
  gravity_symmetry_room}`; `player_phase_split`/`actor_phase_split` pin the
  phase seam. If the lift breaks a `wire_portal_schedule` pin, one of these
  goes RED — no manual schedule inspection needed.
- **Step 6 — ✅ EXECUTED (fable, 2026-07-06 night): the proof shell +
  the engine-resource split it forced.** `ambition_host/tests/
  demo_shell_smoke.rs` boots foundation + `PlatformerEnginePlugins` +
  `PlatformerHostPlugins` + a fixture content plugin for three frames.
  Getting it green surfaced every place the engine group leaned on
  Ambition's assembly, each fixed at its owner:
  - `ambition_runtime::SimCoreResourcesPlugin` minted (in the group,
    right after the sets plugin): ALL engine sim messages + resource
    defaults moved out of the app's `SandboxSimulationResourcesPlugin`
    (now only: catalog/roster install + the data-asset/setup Startup
    chain + profiling report). Everything uses `init_resource`
    semantics — hosts override by insert-before-add.
  - Domain plugins now init their OWN state (rule 5):
    `WorldPrepSchedulePlugin` (+`LdtkHotReloadState` default =
    watcher-off), `LdtkRuntimeSpinePlugin` (+its five indexes +
    `LdtkRuntimeIndex::default()` = the "no LDtk installed" index),
    `CutsceneSchedulePlugin` (+library/bindings/queue/active/advance),
    `TraceSchedulePlugin` (+the portal `BodyTeleported` message,
    idempotent).
  - Content-optional seams: `populate_boss_encounter_registry`
    short-circuits to an empty roster when no game installed boss
    content (`boss_content_installed()`; the missing-install panic
    stays live on any path that RESOLVES a boss);
    `populate_encounter_registry` takes `Option<Res<SandboxLdtkProject>>`
    (RON-only apps have no project; W4 re-routes this through
    `RoomEmission`).
  - `RoomSpec::new(id, world)` public constructor (the generated-room/
    fixture starting point).
  - **What a game/fixture MUST still provide** (deliberately not engine
    defaults): its character catalog (`install_character_catalog`), its
    `RoomSet`/`RoomGeometry`/`ActiveRoomMetadata`, and a Startup system
    in `SimulationSetupSet` that calls `session::setup::
    simulation_world` (spawns the player box). The smoke test IS the
    reference implementation (demos/README.md points at it).

Exit checks: ✅ the smoke shell passes; the app's `plugins.rs` shrank to
content installs + Ambition-specific wiring; rl_sim app tests (the
schedule-shape guard) green; the host boundary test holds ([the windowed
host] imports no `ambition_content`). **E5-finish is COMPLETE — THE DEMO
GATE IS OPEN** (S5/M-track demo apps may mint).

### W1–W4 — the world carve — [opus]

Precondition: none (parallel-safe with E5). The four cards:

- **W1 — invert the `world` upward deps.** Grep `world/` for imports of
  higher-tier types; each becomes a message the sim consumes or a
  parameter the caller passes (the Contact/FrameEvents pattern). One
  commit per inversion; no move yet. **Measured 2026-07-06 (opus) — the
  deps split by KIND, and only one kind is a clean W1 invert:**
  - **Runtime STATE deps (clean W1 inverts — live sim state the IR must
    never name):** ✅ **`rooms/load.rs` DONE (opus 2026-07-06):**
    `load_room_geometry` dropped its `PlayerSafetyState`/
    `PlayerBlinkCameraState`/`DialogState`/`BodyCombat` params +
    `ROOM_DOOR_CAMERA_SNAP_TIME`; it now returns geometry + arrival facts
    only, and the composition tier (`ambition_app` `room_flow::
    apply_room_transition_resets`) applies the four cross-domain resets
    from the returned `arrival_pos`+`edge_exit` (anti-god rule 6: no
    single domain owns the transition, so the composer does).
    Byte-identical. **REMAINING STATE dep:** `rooms/systems.rs
    detect_room_transition_system` reads `ControlledSubject`,
    `SlotInteractionState`, `BodyKinematics`, `PrimaryPlayerOnly` — this
    is NOT an invertible dep, it is a **sim-tier system that merely lives
    under `world/rooms`**; it MOVES to the sim heart at W3 (it detects a
    body/zone overlap and emits `RoomTransitionRequested`). Document the
    move; don't try to invert it.
  - **VOCABULARY deps (the genuine W3 classification — ESCALATED to
    fable, see the W-track feedback block below):** `CharacterBrain`,
    `BossBrain`, `KinematicPath`/`KinematicPathMode`, `RespawnPolicy`
    (all `ambition_characters::actor`) and `DamageVolume`
    (`ambition_combat`) are named pervasively — `world/platforms`,
    `ldtk_world/{surfaces,fields,conversion,entity_converters}`, and the
    `RoomEmission`/room-graph types (`rooms/room_graph.rs`,
    `ldtk_world/conversion/mod.rs`). `world/rooms/specs.rs` +
    `camera.rs` add doc-comment-only mentions (no compile dep).
- **W2 — IR naming in place.** `RuntimeEntityEmission` →
  `RoomEmission` (carrying the S3 `chains` channel); `SpatialSource`
  provenance enum replaces render's `"ldtk "` name-sniff; plain-serde
  derives on `World`/`Block`/`SurfaceChain`/AABB wrappers; the baked
  `ron-room` manifest loader (serialized `RoomSpec` — generated rooms/
  fixtures only; authored space stays backend files). The sanic area
  gains its `ron-room` twin as the IR proof.
- **W3 — the two-crate cut.** `ambition_world` = IR + rooms graph +
  composition + converter REGISTRY (no LDtk dep anywhere in it —
  enforce with a dep test); `ambition_ldtk_map` = the LDtk backend
  (parser, spine, converters that read LDtk types) depending on
  `ambition_world`. Atomic move per the D2 template; every consumer
  repoints in the same arc. Record compile timings before/after.
- **W4 — the ratchet + ADR 0021.** Encounter loading → emissions;
  menu-map/session/settings inversions; schedule-set rename; write ADR
  0021 (authoring-backend-agnostic space) citing spatial-model.md +
  frame-awareness.md. Exit: `ambition_world` builds with zero LDtk in
  its tree, and a fixture "second backend" test constructs a RoomSpec
  purely from IR calls.

#### ✅ W-track — the vocab-arrow question is RULED (Jon + GPT-5.5, 2026-07-06)

Opus escalated this; Jon + GPT-5.5 ruled it. **The doctrine is now in
[`architecture.md`](architecture.md) §4b (canonical) + the Tier-0
schema-vs-component note.** Summary of the ruling (do NOT reopen it):

- **World IR stays PURE** — `ambition_world` names ZERO runtime
  character/combat/projectile/demo types. (This RULES OUT the old option
  "(b) draw the arrow".)
- **Authored maps still declare content** — spawns, the falling-sand
  SPOUT (the canonical example: an authored placement, not a runtime
  hack), hazards. `RoomEmission` carries **authored placement RECORDS
  over closed Tier-0 authored SCHEMAS** (old option "(c)"), NOT runtime
  types and NOT a loose opaque payload (Jon prefers the closed,
  editor-visible schema; hybrid only if a closed schema is infeasible —
  no case seen).
- **`KinematicPath`/`KinematicPathMode` are world/geometry vocabulary**
  (moving-platform paths), mis-homed in `ambition_characters` by history
  (old option "(d)", confirmed) → move to `ambition_world` or Tier-0.
- **World→sim LOWERING seam:** sim/content INTERPRET world records into
  behavior; the arrow is sim/content → world, never reverse; interpreters
  register in [the space IR]'s converter registry; lowering runs at
  room-load.
- **The world is not immutable** — a base+delta seam for permanent
  gameplay change is RESERVED (architecture §4b.5 / §5).

**✅ ALL FIVE SUB-QUESTIONS RULED (fable, 2026-07-06 night — the last-chance
pass). Everything below is now OPUS-SAFE; no design freedom remains, only
execution. Do not reopen; deviations follow vision.md §7 (a genuine
"fable didn't see X" only).**

- **✅ RULED [W-a] — the Tier-0 schema home is `ambition_entity_catalog`
  [the authoring spine], WITH the tier-purity constraint that decides
  what may move whole.** A scout confirmed every type in question is
  already pure serde-able data with zero runtime state
  (`ambition_characters/src/actor/mod.rs`: `CharacterBrain` {Passive,
  Patrol, Guard, Custom}, `BossBrain` {Dormant, PhaseScript, Custom},
  `KinematicPath`/`KinematicPathMode`, the hazard/prop `RespawnPolicy`
  {Never, AfterSeconds, OnRoomReload, Persistent};
  `ambition_combat/src/lib.rs`: `DamageVolume` + `Damage`/`DamageKind`
  and `DamageTeam`). **BUT `ambition_entity_catalog` is serde+ron ONLY
  and stays that way — it may NEVER dep `ambition_engine_core`** (Tier 0
  cannot import Tier 1; the `HitVolume` precedent uses `VolumeShape` +
  plain f32s, never `Aabb`/`Vec2`). That constraint yields the verdicts:
  1. **Move WHOLE to `ambition_entity_catalog::placements`** (one module,
     doc-headed "the authored-placement schema vocabulary — §4b"):
     `CharacterBrain`, `BossBrain`, `DamageKind`, `DamageTeam`, and the
     hazard/prop respawn enum RENAMED `HazardRespawn` (name-collision
     ruling: the ADR-0022 actor `RespawnPolicy` keeps its name and also
     lands in the catalog when E2 relocates the archetype schema; two
     same-named enums may not coexist in the schema module). All are
     dep-free pure enums. Consumers re-import explicitly; NO lasting
     re-export shims (D2).
  2. **`KinematicPath`/`KinematicPathMode` → `ambition_engine_core`**
     (NOT the catalog): they carry `Vec2` points — spatial GEOMETRY
     vocabulary, which is exactly what engine_core owns
     (`World`/`Block`/`SurfaceChain` live there). This satisfies the
     earlier "world or Tier-0" ruling at the correct tier: engine_core
     is below every consumer (the small `ambition_combat` crate
     included), and the W3 world crate deps engine_core anyway.
  3. **`DamageVolume` and `Damage` do NOT move — they DISSOLVE at W2/
     lowering.** `DamageVolume` is an authored hazard-placement record
     in disguise (`id` + `aabb` are RECORD-level fields; the rest is the
     schema). Under the [W-b] shape it becomes `PlacementRecord { id,
     aabb, schema: PlacementSchema::Hazard(HazardSpec) }` with Tier-0
     `HazardSpec { damage: i32, knockback: [f32; 2], kind: DamageKind,
     team: DamageTeam, hitstop_seconds: f32, respawn: HazardRespawn,
     path_id: Option<String> }` (plain pairs, the HitVolume idiom; the
     lowering interpreter converts to `Vec2` once at room load). The
     legacy types are REPLACED, not relocated — delete them when the
     hazard interpreter lands (W-queue step 3).
  4. **The general split line (for FUTURE types):** the Tier-0 schema is
     what the author writes, in plain serde types (numbers/strings/
     enums, `[f32; 2]` for vectors — never kernel types); the spatial
     footprint lives on the placement RECORD (world crate, Tier 2); a
     runtime component may EMBED the schema by value, NEVER mirror it
     field-by-field (reorganize-don't-adapt).
- **✅ RULED [W-b] — TWO stages, ONE pattern, both owned by [the space
  IR].** The two-stage seam is explicit: (1) the EXISTING backend
  converter registry (`ldtk_world/conversion`, keyed by LDtk entity
  identifier) parses backend entities into **authored placement RECORDS**
  on `RoomEmission`; (2) a NEW **lowering registry** (same
  registration pattern, different key: the Tier-0 schema KIND) maps each
  record → live entities at room-load. They are NOT merged into one
  registry — the keys and inputs differ, and merging would couple the
  backend to sim types (exactly what W3 forbids). Pinned API shape:
  ```rust
  // [the space IR] (gameplay_core::world today; ambition_world at W3)
  pub struct PlacementRecord {
      pub id: PlacementId,          // W-d: REQUIRED (LDtk iid / bake-synth)
      pub schema: PlacementSchema,  // the CLOSED Tier-0 enum (§4b.3)
      pub aabb: Aabb,               // authored footprint (pos+size)
  }
  pub type LoweringFn = fn(&PlacementRecord, &mut LoweringCtx);
  // LoweringCtx wraps Commands + room/arrival facts; grows fields by need.
  impl App /* extension trait in the space IR */ {
      fn register_placement_interpreter(&mut self,
          kind: PlacementKind, f: LoweringFn) -> &mut Self;
  }
  ```
  `PlacementKind` is the fieldless mirror of `PlacementSchema`'s variants
  (derive or a `kind()` method). ENGINE interpreters (hazard→combat) and
  CONTENT interpreters (spout→falling-sand) register through the SAME
  call — the registry is open by construction. Duplicate registration for
  one kind PANICS (two owners = an authority bug, anti-god rule 5). The
  room-load call site: the same spawn pass that today hardcodes
  feature spawning (`spawn_room_feature_entities` path) iterates the
  emission's records and dispatches by kind; hardcoded branches convert
  to registered interpreters one at a time (each its own commit).
- **✅ RULED [W-c] — base + ordered DELTA OPS, named `WorldDelta`,
  reserved now, implemented at first need.** Representation: an ordered
  op list per room (`enum WorldDeltaOp { RemoveBlock(GeoId),
  AddBlock(Block) /* minted GeoSource::Delta{op_index} */,
  RemovePlacement(PlacementId), … }` — the op set grows
  variant-by-variant as features land), persisted into the save as a
  patch, replayable. **Ops name geometry by `GeoId` — the durable
  geometry-identity model ruled in collision-and-ccd.md §3.6** (authored
  sources only; carve pieces/split blocks are DERIVED and can never be
  named by an op). NOT a mutable world, NOT save-side geometry
  snapshots (ops are compact, serialize as events for netcode, and
  compose with replay). The effective room = base ⊕ delta, composited by
  the SAME derived-`CollisionWorld`-overlay path transient dynamics
  already use — the delta generalizes that overlay to PERSISTED change.
  **SimView observes ONLY the composited view** (consumers never see
  base-vs-delta). SimView does NOT mirror geometry wholesale: when the
  first permanent-change feature lands, SimView gains a
  `WorldGeometryVersion` fact (tick-tagged bump) and presentation
  re-reads composited geometry through the normal room-(re)load path on
  version change — render already knows how to rebuild a room's visuals;
  reuse that, don't stream polygons through the view.
- **✅ RULED [W-d] — placement ids are REQUIRED NOW at the record layer**
  (`PlacementRecord.id`), because they are effectively free — LDtk
  already provides stable per-entity `iid`s (`config.id == LDtk iid` is
  ALREADY the actor-identity convention, ADR 0022 / the gnu_ton repair),
  and baked `ron-room`/generated rooms synthesize `"{room}:{index}"` at
  bake time — and because [W-c]'s `RemovePlacement(PlacementId)` op and
  netcode N3.1's SimId vocabulary both need them; retrofitting ids under
  saved deltas later would be far more expensive. Consumers (SimView
  identity, replay, fuzz traces, deterministic spawn) adopt lazily.
- **✅ RULED [W-e] — unknown placement = HARD ERROR at room-load
  lowering** (panic naming the schema kind, the placement id, the room,
  AND the list of registered kinds — the catalog-validator precedent:
  fail at the same startup gate a broken catalog reference hits). No
  dev/shipped mode split — Jon: clarity > perfect dev policy.

**OPUS-SAFE — the W execution queue (strict order; each step compiles +
commits alone):**
1. ✅ **W-a moves — DONE (opus 2026-07-06 night).**
   `ambition_entity_catalog::placements` minted with `CharacterBrain` +
   `BossBrain` + `HazardRespawn` (renamed from the actor `RespawnPolicy`) +
   `DamageKind` (moved out of `ambition_combat`) + `DamageTeam` (with its
   `can_damage` matrix + tests); `KinematicPath`/`KinematicPathMode` moved
   to `ambition_engine_core::kinematic_path` beside the geometry vocabulary.
   Every consumer repointed (grep-driven bulk sed of the full paths + the 5
   grouped-brace `use` sites + the two `crate::combat::DamageKind` /
   `ambition_combat::DamageKind` sites); old definitions deleted; NO shims.
   `ambition_entity_catalog` added as a dep of combat/interaction/content/
   sim_view/app. `DamageVolume`/`Damage` stayed put (they dissolve in step 3
   — verdict 3). Gate green: entity_catalog/engine_core/characters/combat/
   interaction unit suites, gameplay_core lib 1175, content 64, sim_view
   boundary, app build clean.
2. **W2 payload:** `RuntimeEntityEmission` → `RoomEmission` carrying
   `Vec<PlacementRecord>` (the [W-b] shape) + the S3 `chains` channel +
   `SpatialSource` provenance + plain-serde derives + the `ron-room`
   loader (card above, unchanged).
3. **Lowering registry:** land `PlacementSchema`/`PlacementKind`/
   `LoweringCtx`/the registry + the [W-e] hard error; convert ONE
   hardcoded spawn branch (the falling-sand spout is the canonical first,
   or hazards if the spout is blocked) as the proof; convert the rest
   branch-by-branch.
4. **W3 cut** (card above): `ambition_world` + `ambition_ldtk_map`;
   `detect_room_transition_system` moves to the sim heart (it is a
   sim-tier system, W1 finding); dep-tests (`ambition_world` names zero
   runtime crates AND zero LDtk).
5. **W4 ratchet + ADR 0021** (card above).

### E1a–E1e — persistence → audio → dialog → dev_tools → menu — [opus; E1a fable-specced]

Strictly ordered; menu LAST. Per card: mint the crate, move the module
(D2: one atomic move after in-crate cleanup), repoint consumers, delete
the facade, run the gate.

- **E1a `ambition_persistence`** (persistence/ 4.5k + host/ + quest/):
  owns *what is stored and its serde shape*. The settings **IR** (which
  renders/pages/curates) STAYS BEHIND for E1e; persistence exposes
  plain typed settings the IR reads. Exit: zero imports from menu/UI
  code (dep test).
- **E1b `ambition_audio`** (audio/ + music/, 1.8k): mechanical.
- **E1c `ambition_dialog`** (dialog/ 2.2k): runtime + lint machinery;
  the game's Yarn BINDINGS stay sim-side (they reference actor state).
- **E1d `ambition_dev_tools`** (core dev/ 3.0k + app dev/ 2.7k): one
  crate, feature-gated overlays; DevToolsPlugin moves whole.
- **E1e `ambition_menu`** (core menu/ 3.2k + app menu/ 10k) — a
  THREE-way split (amended 2026-07-06, the extension-crate ruling in
  architecture.md Tier 6): (1) menu model + settings IR + host stack +
  the plain GRID backend → [the menu stack] (deps
  `ambition_persistence` — the layering that dissolves the god-dep);
  (2) the lunex **kaleidoscope backend** → `game/
  ambition_menu_kaleidoscope`, the FIRST extension crate (engine-only
  deps, boundary-tested, optional for any game incl. Ambition);
  (3) Ambition's menu content stays content-side. The
  `ambition_touch_input` upward-dep inversion rides this card; C3
  (in-game character select over the wear seam) lands here or is
  explicitly closed.

### E2 — the combat/projectiles carve — [opus; back-edges PRE-CLASSIFIED by fable]

Precondition: none, but coordinate with E5 step 2 (the schedule plugin
moves first; the TYPES move here).

1. ✅ **The back-edge inventory + classification is DONE (fable,
   2026-07-06 night — grep of `crate::features::` in `combat/`,
   non-test). Execute each verdict; if the code has drifted, re-grep and
   match the nearest verdict below — do NOT invent a new category:**
   - **`CenteredAabb` (moveset, hitbox, tests)** — a re-export ALIAS;
     the type is `ambition_engine_core::CenteredAabb`. Verdict: repoint
     the import to engine_core. Pure path fix.
   - **`HitEvent` / `HitTarget` (moveset, damage, bus)** — combat
     VOCABULARY living features-side. Verdict (a): the DEFINITIONS move
     into `combat/` (then travel with the crate); `features`/actors
     re-import from combat — the legal actors→combat arrow.
   - **`FriendlyFire`, `FactionRelations` (on_hit, hitbox)** — faction/
     targeting policy resources. Verdict (a): combat owns targeting
     (architecture Tier-2 row) → move both into `combat::targeting`;
     the WorldPrep init moves with them (rule 5).
   - **`Option<&ActorConfig>` (moveset, hitbox — the CM1 weight read)**
     — a genuine sim fact from the actor domain; combat may NOT import
     the sim heart. Verdict (b): mint a combat-owned component (e.g.
     `CombatTuning { weight, … }`) that actor SPAWN writes from the
     archetype (actors→combat, legal); the two queries read it instead
     of `ActorConfig`. Byte-parity: same values, new carrier.
   - **`SetFlagRequested`/`QuestAdvanceRequested` (bus, pickups)** —
     progression vocabulary. Verdict (a-down): these messages belong to
     [the saved shapes] (`ambition_persistence` owns flags/quest rules)
     — move at E1a if it lands first, else leave in features and record
     the arrow combat→persistence as the target; combat only WRITES
     them.
   - **`SwitchActivated`, `GameplaySfxRequested` (bus)** — encounter/
     sfx vocabulary. Verdict (a-down): `GameplaySfxRequested` →
     `ambition_sfx` (the effect vocabulary crate); `SwitchActivated` →
     `ambition_encounter` at E-enc (combat only writes it).
   - **`GameplayBanner` (damage)** — a UI resource written directly
     from combat. Verdict (b): write the EXISTING
     `GameplayBannerRequested` message instead; only the UI layer reads
     the resource. (Kills a combat→UI write.)
   - **`FeatureEcsWorldOverlay` (attack)** — already DEFINED in
     `combat/overlay.rs` (features re-exports it) and carries only
     engine_core types (`Vec<ae::Block>` + gate solids). Verdict: the
     overlay type + rebuild are geometry COMPOSITION → they move to the
     world crate in the W-track (ledger row `world_overlay`); until
     then, repoint the features-path references to `combat::overlay`
     (they're path residue). Combat systems keep reading the resource;
     post-W3 the arrow is combat→world which is NOT drawn — at that
     point the composited solids become a system PARAM (the same
     inversion as W1).
   - **`FeatureSimEntity` (hazards)** — room-scoped-lifecycle marker.
     Verdict (a-down): the marker is lifecycle vocabulary → move to
     `ambition_platformer_primitives::lifecycle` (where `SceneEntities`
     already lives); everything re-imports from there.
   - **`select_actor_targets` (components/actors doc-comment)** —
     comment-only; rewrite the sentence when the file moves.
   **Note (Q31, 2026-07-06):** for any combat type that is BOTH an
   authored placement schema AND a runtime component (`DamageVolume` is
   the case — authored hazards vs the live hitbox), the AUTHORED-schema
   half follows the [W-a] ruling (→ `ambition_entity_catalog::
   placements`); the runtime half moves to `ambition_combat` here. Don't
   merge the two decisions — E2 owns the runtime move, W-a owns the
   authored schema.
2. ✅ **DONE (opus 2026-07-06 night) — the in-place verdicts, one commit
   each, all byte-parity, all INSIDE gameplay_core:** (1) `CenteredAabb`
   off the `crate::features` hub → `ae::CenteredAabb`; (2) `HitEvent`/
   `HitTarget` combat sites off the hub → `crate::combat::events::`;
   (3) `FactionRelations`/`FriendlyFire` init owned by combat
   (`combat::targeting::init_targeting_resources`, WorldPrep invokes it —
   rule 5); (4) minted `combat::CombatTuning { weight }`, written at the two
   actor-spawn choke points (`into_components` + `boss_actor_cluster`), the
   hitbox weight read converted off `Option<&ActorConfig>` (the moveset
   sprite-id read + attack cooldown-mult read are a DIFFERENT field/concern,
   not this verdict — left for the atomic move); (5) `damage.rs` writes
   `GameplayBannerRequested` instead of the `GameplayBanner` resource;
   (6) `FeatureEcsWorldOverlay` combat sites → `combat::overlay`;
   (7) `FeatureSimEntity` → `ambition_platformer_primitives::lifecycle`.
   Gate green each commit (gameplay_core lib 1175; full app rl_sim suite on
   the two behavior-touching verdicts 4+5; only the documented
   `unified_melee::a_hostile_actor` feel-RED fails). The remaining combat
   upward reads (`features::HitSource`/`HitMode`/`HitKnockback`/
   `ActorFaction`/`world_with_sandbox_solids`/`ENEMY_ATTACK_COOLDOWN`, the
   moveset/attack `ActorConfig` non-weight reads) are combat's own
   vocabulary or other-domain facts to resolve at the ATOMIC move (step 3,
   RESERVED) — not among the pre-classified in-place verdicts.
3. Atomic moves: `combat/` (minus `overlay.rs` → W-track; minus
   `boss_clusters.rs` which dissolves in E6) → `ambition_combat`;
   `projectile/` + `enemy_projectile/` → `ambition_projectiles` (deps:
   combat). Direction ruled: **features → combat, never the reverse.**
4. Only after the move: further combat-model slices (CM6+) land in the
   new crate.

### E3 — `ambition_sprite_sheet` absorb + the asset-root flip — [opus]

Precondition: G1 landed (it did). Moves `character_sprites/` (4.3k) +
`boss_encounter::{sprites, attack_geometry}` into the existing
`ambition_sprite_sheet`; carries the asset-root flip and the blocked
residue cluster: ParallaxTheme #6, `pirate_weapon` #7,
`ProjectileVisualKind::{Apple,Glider,Lasersword}` art descriptors, the
six `BossSheetSpec` statics. Then the sprite-adjacent bug queue homes
here: §7.3 boss-generic-sheet (FIRST add the `boss_sprites.len()`
startup log + downgrade `MissingAssetPolicy::SilentPlaceholder` to a
logging policy — then run), §7.8 shrine/glider rect drift, §7.4 modal
body-morph rows (a `BodyMode` selects a sheet-supplied sprite-state;
deletes the morph-ball overlay + hide-toggle).

### E4 — [the observation boundary] + the render edge cut — [★fable executes steps 2–4; opus does step 1]

*(Re-graded 2026-07-06 per Jon: the hardest decompositions are fable
work. This is the riskiest cut — the sim/presentation boundary.
Historical note: the old W3/E2 fable-escalation rule is RETIRED — E2's
back-edges are pre-classified in the E2 card and W-a..W-e are ruled; the
post-fable protocol lives in tracks.md.)*

**Steps 1–2 are DONE (2026-07-06):** the vfx-message types already
lived in `ambition_vfx` (render's `fx` was a re-export facade); every
sim-side consumer now imports from `ambition_vfx` directly, and the
scout ran — verdict: **the carve can start; `ambition_portal_
presentation` is ALREADY below the boundary (zero gameplay_core refs);
render imports ~103 distinct gameplay_core symbols; render WRITES sim
state in exactly three places** (the `CameraEaseState` ResMut in
`camera_follow`; `FeatureName` inserts on render-spawned props in
`rendering/world.rs`; the `BossAnimator` insert in
`rendering/actors/boss.rs` — the E6(a) back-edge) **plus render's
plugin registering sim mutators** (`advance_actor_anim_overlays`,
`rebuild_actor_anim_index`, and nine portal glue systems).

**Step 3 — the pre-inversion queue (scouted 2026-07-06; each is one
committable slice; render-side reads become SimView fields):**

1. `BodyKinematics` reads (sync_visuals/animation/camera/fx/items/
   projectiles/pirate_weapon) → `ActorRenderView { pos, velocity,
   size, facing }` (AJ14 pos+velocity land here). ⏳ **player half DONE
   (fable 2026-07-06): `BodyPoseView` component** (pos/vel/size/
   base_size/facing/roll/stance/gravity/anim/flash/hp/morph/charge),
   rebuilt in `FeatureViewSync` (`pose_view.rs`); sync_visuals player
   branch, animate_player, morph_ball, charge indicator, placeholder
   override, player hit-flash + debug health bar are pure consumers;
   `ShieldRingsView` pools every raised shield (player+actor). Actor-
   side kin reads in fx/items/pirate_weapon remain (slices 11–12, 18).
2. ✅ The `ActorSpriteData` mega-QueryData in render is GONE (fable
   2026-07-06): hit_flash reads `FeatureView.hit_flash_secs` (+
   `BodyPoseView` for player bodies); deep_dream reads
   `ActorRenderView.dream_seed` + name.
3. ✅ `BodyAnimFacts`/`BodyMelee`/`PlayerBlinkCameraState`/`BodyCombat`/
   `Body*State` cluster reads: the actor half landed as slice 19
   (`ActorAnimIndex`); the PLAYER half landed with `BodyPoseView`
   (fable 2026-07-06) — `pick_player_anim` now runs sim-side only.
4. ✅ `ActorRoll` → `BodyPoseView.roll_angle` (player; actors already
   rode `FeatureView.rotation_rad`).
5. ✅ `BodyHealth`/`BodyCombat`/`Health` reads (health/hit_flash/
   nameplates/boss/overlays) → `FeatureView.{alive, hit_flash_secs,
   hp_current, hp_max, training_dummy}` (fable 2026-07-06); the hud
   half rides slice 6.
6. ✅ `BodyWallet`/`BodyHealth`/`BodyMana` (hud) → `sim_view::
   PlayerHudFacts` (fable 2026-07-06); `regen_player_mana` moved
   SIM-side (a mutator never lives in presentation).
7. ✅ Boss internals (`BossConfig`/`BossClusterRef`/`BossPhase`/`Brain`/
   `BossAttackState`) → `BossFrameIndex` (fable 2026-07-06): per-boss
   `BossAnimState` + combat AABB + the sim-computed hazard-column lane
   (same volume math as damage); `animate_bosses`, the gradient-lane
   visual, and the boss health bar are pure consumers. Dissolves into
   the actor index at E6(b).
8. The render-inserted `BossAnimator` → E6(a): sim-owned
   `BossAnimFrame`; render stops inserting.
9. ✅ Live feature-marker queries (encounter mobs, staged actors,
   post-boss NPCs, reward chests) → `sim_view::DynamicFeatureViews`
   (fable 2026-07-06); `FeatureEcsWorldOverlay` (lock walls) remains a
   render-read resource — it is already a derived read-model.
10. Render-inserted `FeatureName` on props → sim inserts at room load
    (or a render-local `PropName`).
11. ✅ `HeldItem`/`GroundItem`/`HeldProjectile` → `sim_view::
    {HeldItemView, GroundItemsView, HeldShotsView,
    WieldedGunSwordsView}` (fable 2026-07-06) — pirate gun-sword hand/
    aim resolved sim-side too.
12. ✅ `ActorControl` read in item_visuals → `HeldItemFact.aim`
    (fable 2026-07-06).
13. ✅ `sim_view::ProjectileView { kind, pos, vel, size }` component on
    live projectiles (removed on pooled reuse); charge tier rides
    `BodyPoseView.charge_tier` (fable 2026-07-06). Residue: the
    visual↔projectile link is still `Entity`-keyed (deterministic
    spawn ids arrive with netcode N3.1).
14. ✅ `PlayerMark` → `sim_view::MarkBeaconsView` (fable 2026-07-06).
15. ✅ `GravityFlipSwitch`, `HealShrine` → `sim_view::
    {GravitySwitchesView, ShrinesView}`; the `ShrineActivationPulse`
    timer now ticks SIM-side (the render write is dead) and render
    reads it read-only (fable 2026-07-06).
16. ✅ `ControlledSubject` reads (camera/hud/fx/items/nameplates) are
    GONE from render (fable 2026-07-06): each consumer's view carries
    the controlled-body resolution as a FACT (`PlayerHudFacts`,
    `HeldItemView`, `NameplateFact.controlled`, `BlinkPreviewFact`,
    the camera resolve) — the sim resolves the subject once per
    domain, render never sees an `Entity`.
17. ✅ **DONE (fable 2026-07-06): the one render WRITE inverted.**
    `CameraObservationPlugin` (gameplay_core `camera_snapshot`, in the
    engine group) resolves the follow snapshot as a TAIL OBSERVER after
    `CoreSimulation` — the only `CameraEaseState` writer; render's
    `camera_follow` applies a COPY (portal-continuity deltas + shake).
    Observer-input resources: `CameraViewport` (host publishes),
    `CameraExtraClamp` (portal continuity bridges same-frame). NOTE for
    the carve: `PresentationSync` is nested INSIDE `CoreSimulation`, so
    post-sim observers anchor `.after(CoreSimulation)`, never in that
    set. AJ14's `observer_velocity` field rides the sim_view mint (the
    snapshot builder now lives sim-side, so it's a field addition).
18. ✅ fx blink preview → `sim_view::BlinkPreviewFact { active, target,
    precision, body_min_extent }`, resolved sim-side with the SAME
    destination math the actual blink uses (fable 2026-07-06); render
    only draws the ember ring.
19. ✅ **DONE (fable 2026-07-06): the extraction systems moved
    sim-side.** `FeatureViewSyncSchedulePlugin` (already the
    observation-rebuild plugin, in the engine group) now owns
    `ActorAnimIndex` + the `(advance_actor_anim_overlays,
    rebuild_actor_anim_index)` chain in the `FeatureViewSync` tail;
    render's `PresentationVisualAnimationPlugin` is a pure consumer
    (its init + registrations deleted). Ordering preserved for free:
    `PresentationVisualSync.after(FeatureViewSync)` already pins
    `animate_characters` to same-frame reads. Headless builds now
    compute the pose read-model — that is the POINT (clip+phase is a
    SimView fact: netcode confirmation, brain move-phase reads,
    per-observer views).
20. ✅ **DONE (fable 2026-07-06, executed as pinned):** the host
    adapter's `PortalObservationPlugin` (host-added, portal_render-
    gated) registers the glue in the public sim-side
    `PortalObservationSet`; render keeps exactly ONE set-to-set
    constraint (`PortalPresentationSet.after(PortalObservationSet)`).
    The audit ruled `tag_portal_scene_bodies`'s `.after(sync_visuals)`
    pin STALE (it tags SIM bodies — `PlayerVisual` + `PortalSceneBody`)
    — dropped. `load_portal_gun_art` + F7/F10 dev toggles ride the same
    plugin.

**Step 4 — ✅ MINTED (fable 2026-07-06):** `crates/ambition_sim_view`
= [the observation boundary]: `camera_snapshot` (types + the
`CameraObservationPlugin` resolve), `view_index` (feature/actor/boss/
nameplate indexes), `anim_index` (`ActorAnimIndex` + `BossFrameIndex`
+ `ActorSpriteData`), `pose_view` (`BodyPoseView` + shield rings),
`facts` (hud/items/marks/shrines/gravity/gun-swords/projectiles/
dynamic-features/blink), + `FeatureViewSyncSchedulePlugin` and
`SimViewPlugin` (added by the runtime group). RULING pinned at the
mint: **camera-EASE state stays sim-side** (`time/camera_ease`) — the
boss shake + portal-continuity writers are sim systems; the ease
RESOURCE is sim state and only its RESOLVE observes. The sim→view
contract tests moved to `ambition_sim_view/tests/view_contract.rs`
(the dev-dep cycle gives gameplay_core's test build a different type
universe). **Step 5 — the flip:** partially enforced NOW by
`ambition_render/tests/observation_boundary.rs` (render sources must
never name ~45 live sim-STATE types — whole-identifier match,
comment-stripped); the full dep-flip (render drops gameplay_core
entirely) stays gated on E1 (menu/dev/persistence), E3
(character_sprites), E-assets (GameAssets), and the rooms/world
carve — those are the remaining render→gameplay_core imports, all
vocabulary/assets, not sim state.
**✅ SimView authority CONFIRMED (Jon, 2026-07-06, roadmap Q32):** SimView
IS the presentation/observation boundary; presentation migrates toward
SimView/observation facts, not raw sim reads, and architectural CHURN is
ACCEPTED when it removes long-term coupling (the long game). So Step 5 is
now **OPUS-SAFE sequencing, not a design question** — it proceeds
mechanically as each gate (E1/E3/E-assets/W) lands; no fable ruling
needed. (The SimView-world question is RULED with [W-c]: SimView observes only
the base⊕delta COMPOSITED view and gains a `WorldGeometryVersion` bump
fact when permanent change lands — never wholesale geometry mirroring.)

#### E4 design sketch (pre-solved; do not re-derive)

The view types already exist — the carve RELOCATES and SEALS them, it
does not invent them: `FeatureViewIndex`, `ActorRenderView`/
`ActorRenderIndex`, `BossRenderView`/`BossRenderIndex` (the boss pair
dissolves into the actor index when E6(b) finishes) — all in
`features/ecs/view_index.rs` — plus `CameraSnapshot2d` + its resolve
inputs (`camera_snapshot.rs`). Target shape in [the observation
boundary]:

```rust
/// Rebuilt every sim tick by extraction systems that run LAST in the sim
/// schedule. Presentation reads ONLY this. Plain data, no Entity borrows
/// beyond opaque ids, no Handle<T>, no interior mutability — snapshot-safe
/// by construction (netcode N3.1 serializes these for free).
pub struct SimView {
    pub tick: u64,                       // confirmed-tick tag (rollback-ready)
    pub actors: ActorRenderIndex,        // per-body: pos, VELOCITY (AJ14),
                                         // facing, clip+phase, tint/flash facts
    pub features: FeatureViewIndex,
    pub camera: CameraSnapshot2d,        // + observer_velocity (AJ14)
    pub events: Vec<PresentationFact>,   // sfx/vfx/shake facts, tick-tagged
}
```

Rules the sketch fixes: extraction systems are FUNCTIONS of sim state
(no caching across ticks except double-buffer); `PresentationFact` is
the ONE event channel presentation consumes (the CM5 per-move sfx/vfx
events flow through it; rollback dedups by tick); render never queries
a sim component type — the boundary test greps `ambition_render` for
gameplay-core/actors types and fails on any hit.

**Identity & ownership (pinned 2026-07-06):** view rows key by the SAME
stable-id vocabulary the snapshot registry uses (netcode N3.1: actor
`config.id`, player slots, deterministic spawn ids) — one identity
system, two consumers; render maps its presentation entities off those
ids, never off sim `Entity` values. `PresentationFact` dedup identity is
the triple `(tick, source SimId, kind)` — that is what resim suppression
keys on. Render-spawned helper props are PRESENTATION CACHES keyed off
view rows (despawn/respawn freely; never readable by sim); anything the
sim reads must be a sim fact — a render-inserted component the sim
queries (the old `BossAnimator` shape) is the boundary violation this
carve exists to kill.

#### D-C design sketch — the mode scope (pre-solved)

```rust
// ambition_world (RoomMetadata):   pub mode: Option<String>,  // merge: first Some wins
// ambition_runtime:
pub fn in_mode(name: &'static str) -> impl Condition { /* reads ActiveRoomMetadata */ }
#[derive(Component)] pub struct ModeScopedEntity(pub String); // despawned when the mode deactivates
```

A rules crate attaches every system `.run_if(in_mode("sanic"))` when
hosted, or unconditionally when standalone — the APP chooses via
`SanicRulesPlugin::hosted()` vs `::global()` (a constructor flag, not
two plugins). Mode resources live on a mode-owner entity carrying
`ModeScopedEntity`, so zone exit cleans up by the same sweep
`RoomScopedEntity` uses (generalize that sweep, don't duplicate it).

### E6 — the boss tail — [opus]

Precondition: E3 (sprite moves) recommended first. Fully enumerated:
(a) sim-side `BossAnimFrame` (the sim stops reading a render-inserted
animator component); (b) remaining `BossAnim` rows → `CharacterAnim`
for non-gnuton bosses (BLIND visuals, frame-sample pins); (c)
`BrainSnapshot.target_pos` retirement (the boss brain consumes its
view/target directly); (d) DECIDE the two recorded deep folds (the
no-boss-arm integrate fold; `BossAttackIntent` → general move-intent
folding the boss brain-tick into `tick_actor_brains`) — **"cheap" is now
BOUNDED (fable): attempt each fold on a branch, time-boxed to half a
session; it counts as cheap iff the diff stays ≤ ~200 LOC net, adds NO
new seam/adapter types, and the boss suites pass unmodified. Miss any
bound → revert and write the permanent-policy comment at the code site
naming which bound failed.** Either closes the item. Plus the deferred
teardown: fused `gnu_ton` profile + `sync_boss_split_overlay` +
`BossOverlayLayer` + split z-consts (retarget the referencing tests to
the linked-pair arena first).

### E7 / E8 — residue + the workspace re-home — [opus/sonnet]

E7: the `ambition_actors` rename (pending Jon's Q2 — **default ruling
so this never stalls: if Q2 is still unanswered at execution time,
`ambition_actors` IS the name**; a later rename is one mechanical
sweep), the features-hub
facade dissolution, **and the workspace re-home** (architecture §1):
`ambition_content` + `ambition_app` move from `crates/` to `game/`,
demo pairs live under `demos/` — a mechanical `git mv` + workspace-
members + CI-path slice that makes the engine/game/demo split visible
in the filesystem (do it LAST in D-A; it touches every path reference
once). E8: `inventory_ui/` → [the stuff kit]; the `time/` residue stays
by measurement. Plus the remaining crit-3 slices: the
`dialog/speech_sfx.rs` voice table → a content voice-profile registry;
the `StartingCharacter` worn-sheet residue (`PLAYER_CHARACTER_ID` /
`PLAYER_FILE_ROOT` in `character_sprites/attack_hitbox.rs`).

### E-enc / E-assets — the quiet absorbs — [opus/sonnet]

`encounter/` (+ `features/ecs/encounter_rewards.rs`, + boss_encounter's
`encounter_script`/`rewards` halves) → `ambition_encounter`; `assets/` →
`ambition_asset_manager`. Both are low-entanglement mechanical moves per
the ledger; schedule them as fillers between the big carves.

### App residue — the progression schedule split — ✅ DE-WOVEN (opus 2026-07-06); move-to-group is the follow-up

`ProgressionSchedulePlugin` interleaved engine boss-encounter systems with
content quest/cut-rope systems. **Done:** the engine chain is now
content-free — the five wedged content systems (`setup_cut_rope_encounter`,
`spawn_cut_rope_victory_npc`, `grant_quest_completion_rewards`,
`populate_quest_registry`, `gate_gnu_ton_arena_ladder`) hang on labeled
slots (`ContentEncounterScriptSet` / `ContentEncounterVictorySet` /
`ContentQuestRewardSet` in `boss_encounter`, host-anchored at the exact
former positions) and are registered by `AmbitionBossContentPlugin` /
`AmbitionQuestContentPlugin`, same shape as the combat-schedule
(`ContentSpecials`/`ContentFlavor`) + reset slots. The quest EVENT pump
(push/apply) stayed engine (it was never content — a content:: re-export).
Ordering preserved byte-for-byte (replay-fixture determinism guard +
boss_lifecycle 8/8 green). **Follow-up ✅ DONE (E5 step 5, 2026-07-06 night):**
`ProgressionSchedulePlugin` moved into `ambition_runtime` and rides
`PlatformerEnginePlugins`.

## Phase D-B — the post-carve `ambition_actors` and the navigability standard

After D-A, the residue is the actor sim core (~30–35k): spawn/tick/
perception/damage-routing, player systems, ability kit, body modes,
session, schedules, view-index builders. Rulings:

- **Re-measure before further splits** (U1 stands). The likely-clean
  further carve if measurement supports it: the traversal-ability kit
  (blink/dive/grapple/possession) — it reads the controlled-subject seam
  and kinematics, not the spawn machinery. Do NOT pre-commit.
- **The navigability standard applies INSIDE the crate** (this is where
  "agents can work cleanly" is actually won, and it applies to every
  engine crate): every module ≤ ~1.5k lines with a header stating its
  ONE concern, its authoritative state, and its seams; `features/mod.rs`
  hub-glob patterns dissolve into explicit imports (standing rule); the
  schedule vocabulary documented in one place; a `MODULES.md` map at the
  crate root maintained by the same rule as TODO discipline. Slices are
  mechanical [sonnet] once E-track lands.

## Phase D-C — the demo-hosting seam (ambition runs the demos)

The vision §5 requirement forces one more decomposition artifact — the
**scoped game-mode pattern**: a demo's rules crate exposes
`<Demo>RulesPlugin` whose systems are gated on an area/room tag (a
`RoomMetadata` field, the C1 `gallery` pattern generalized to
`mode: Option<String>`), not on global state. `ambition_app` adds the
demo content crates + mounts their zones; the standalone demo app adds
the same rules plugin globally. Design detail in
[`../demos/README.md`](../demos/README.md); the engine-side slice is the
room-scoped run-condition helper (`in_mode("sanic")`) + the mode field.

## Exit criteria (the whole playbook)

1. `ambition_gameplay_core` no longer exists (renamed residue included);
   every crate in architecture.md's stack is real with imports flowing
   downward (enforced by the boundary tests).
2. The named-content grep over engine crates hits zero (test fixtures
   allowed only under `cfg(test)`).
3. A demo app builds from runtime+host groups + its content crate with
   zero engine edits (the oracle, executable).
4. Workspace green: `cargo test --workspace --all-targets --features
   rl_sim` (the one documented feel-reserved RED allowed).
5. Compile receipts: hot-path incremental rebuild (touch a combat file →
   rebuild) measurably below the monolith baseline recorded before D-A.
