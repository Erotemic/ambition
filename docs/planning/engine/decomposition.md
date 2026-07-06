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
- **Step 5 — mint [the windowed host]** (`ambition_host`, new crate;
  MAY dep render/input/leafwing/gameplay_core; must NOT dep
  ambition_content). Move, each as its own plugin inside the group
  (anti-god rule 2): `register_player_input_systems` (content-free
  after step 4), the engine-generic part of
  `register_player_simulation_systems` (the app-local
  `player_clone`/`apply_home_reset_policy`/`sync_player_presentation`
  pieces STAY app-side), `wire_portal_schedule` (behind the forwarded
  `portal` feature, after `PortalPlugin`),
  `register_room_transition_systems`, the camera follow/shake cluster,
  and `add_input_plugins`. Preserve the landmines: the portal wiring
  pins sets against NAMED systems (`collect_gravity_zones`,
  `integrate_sim_bodies`, …) and must run after the sets plugin.
- **Step 6 — the proof shell.** Create a demo-shell smoke test:
  foundation + `PlatformerEnginePlugins` + `PlatformerHostPlugins` + a
  ~20-line fixture content plugin → `app.update()` runs one frame
  without panic. This is the card's exit AND the permanent regression
  guard for the demo gate.

Exit checks: the smoke shell passes; the app's `plugins.rs` shrinks to
content installs + Ambition-specific wiring; rl_sim app tests (the
schedule-shape guard) green; boundary test extended: [the windowed host]
imports no `ambition_content`.

### W1–W4 — the world carve — [opus]

Precondition: none (parallel-safe with E5). The four cards:

- **W1 — invert the ~13 `rooms` upward deps.** Grep
  `world/rooms` for imports of player/combat/features types; each
  becomes a message the sim consumes or a parameter the caller passes
  (the Contact/FrameEvents pattern). One commit per inversion; no move
  yet.
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

### E2 — the combat/projectiles carve — [opus]

Precondition: none, but coordinate with E5 step 2 (the schedule plugin
moves first; the TYPES move here).

1. Inventory the ~23 `combat → features` back-edge refs (mechanical
   grep; commit the list into the PR description).
2. Classify each: (a) combat VOCABULARY living features-side (hitbox/
   hit-event/volume types) → move combat-ward; (b) genuine sim facts →
   invert to parameters/read-model. No (c) — an unclassifiable ref goes
   to tracks.md as a design question, work continues on the rest.
3. Land (a)+(b) as compiling steps INSIDE gameplay_core (the cycle dies
   while iteration is cheap).
4. Atomic moves: `combat/` (minus `world_overlay.rs` → W-track; minus
   `boss_clusters.rs` which dissolves in E6) → `ambition_combat`;
   `projectile/` + `enemy_projectile/` → `ambition_projectiles` (deps:
   combat). Direction ruled: **features → combat, never the reverse.**
5. Only after the move: the combat-model slices (CM1–CM7) land in the
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
work. This is the riskiest cut — the sim/presentation boundary. Related
escalation rule: W3's two-crate cut and E2's back-edge classification
escalate to fable at the FIRST genuinely ambiguous item, not the third.)*

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
16. `ControlledSubject` reads (camera/hud/fx/items/nameplates) →
    `SimView.controlled_body: Option<BodyId>` — ONE field, five
    call sites.
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
18. fx blink preview (`MovingPlatformSet` + composed world) → a
    sim-computed `BlinkPreviewFact { target_point, valid }`.
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
20. The portal glue systems render registers (`sync_portal_world_frame`,
    `sync_portal_viewer`, `sync_portal_camera_continuity_focus`,
    `sync_portal_debug_overlay_to_f1`, `tag_portal_scene_bodies`,
    `portal_dev_toggle_system`, `portal_convention_toggle_system`, the
    `load_portal_gun_art` startup) → the portal adapter side (with E7).
    **Design pinned (fable 2026-07-06):** the blocker is the ordering
    pins against render-internal labels (`PortalPresentationSet`,
    `sync_visuals`, `animate_player`). The inversion: gameplay_core's
    portal adapter declares a public sim-side `PortalObservationSet`,
    registers the glue there in ITS plugin, and render keeps only ONE
    set-to-set constraint (`PortalPresentationSet
    .after(PortalObservationSet)`) — a label dependency, not system
    registration. `tag_portal_scene_bodies`'s `.after(sync_visuals)`
    pin means it reads render-spawned visuals: audit it first; if it
    tags render entities it is PRESENTATION (stays, renamed); if it
    tags sim bodies the pin is stale. [opus executes against this
    paragraph]

**Step 4 — mint the crate:** `ambition_sim_view` takes
`camera_snapshot.rs`, `view_index.rs`, the anim index, camera-ease;
builders stay functions-of-inputs; ONE registered full-screen
post-pass seam stays. **Step 5 — the flip:** `ambition_render` deps =
sim_view + foundations + vocabularies, NOT gameplay_core; boundary
test enforces it forever. Slices 1–16 are individually committable
[opus with this table]; 17–20 and the final flip are the [★fable]
part.

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
folding the boss brain-tick into `tick_actor_brains`) — execute if the
G-track left them cheap, else document as permanent policy with
rationale at the code site; either closes the item. Plus the deferred
teardown: fused `gnu_ton` profile + `sync_boss_split_overlay` +
`BossOverlayLayer` + split z-consts (retarget the referencing tests to
the linked-pair arena first).

### E7 / E8 — residue + the workspace re-home — [opus/sonnet]

E7: the `ambition_actors` rename (pending Jon's Q2), the features-hub
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
boss_lifecycle 8/8 green). **Follow-up:** the now-content-free engine
`ProgressionSchedulePlugin` can move from the app into the runtime group
(`ambition_runtime`) — a trivial relocation now that it names no content
("assemble with what exists; tighten as carves land").

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
