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
