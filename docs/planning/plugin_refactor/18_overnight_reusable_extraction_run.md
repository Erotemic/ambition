# Stage 18: Overnight reusable-crate extraction run

**Status:** IN PROGRESS (started 2026-06-08 evening). Autonomous long run on `main`.
**Goal:** factor serious *reusable* code out of `ambition_sandbox` into reusable
crates/plugins, plus structural cleanups — working through as many backlog items as
the night allows. Each item is an independent, build-green, fully-gated commit.

This is **Phase 2** of the content-crate path (extract the reusable machinery so a
content crate + `ambition_sandbox` rename become possible later). Owner directive:
"big bang shotgun style… give ourselves a big list and work through as many as we
can overnight."

**North star for every extraction:** move toward *a space where agents can add new
content and reuse the systems to build games without getting bogged down in the
details.* So each crate pulled out must be a **clean reusable plugin** — a documented
public API, ergonomic `app.add_plugins(XPlugin)` composition, and no leakage of
Ambition-specific content. Relocating code is not enough; the result must be something
a *different* game (or an agent building one) could drop in. Be bold (shotgun), lean
on the differential test net ([[feedback_bias_toward_executing_big_refactors]]).

---

## Autonomous-run rules (read first)

- **Never stop for questions.** Work around blockers; if an item's clean form is
  infeasible, take its stated fallback, record why in §Progress, and move to the next
  item. Do not block the run.
- **Commit directly to `main`** ([[feedback_work_on_main]] — no feature branches),
  one commit per item, `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`
  trailer. Explicit staging only — **never `git add -A`**.
- **Hands off** `crates/ambition_sandbox/src/dialog/**` and
  `crates/ambition_sandbox/src/dev/dev_tools.rs` (another agent owns them).
- **Crate extractions use the FACADE pattern** (proven by `ambition_engine_core`):
  move code to the new crate, then keep `crate::<name>` as a thin re-export
  (`pub use <new_crate>::*;`) so the ~dozens of inbound `crate::<name>::…` sites need
  ZERO churn. Low-risk + reversible.
- **Behavior changes that need a human eye are OUT OF SCOPE** (enemy-AI *feel*, visual
  tuning). This run is structural: moves, extractions, dedup, gated by the test net.
  Anything that would change gameplay feel is teed up as a follow-up, not shipped blind
  ([[feedback_headless_diff_reference_first]]).
- Keep §Progress live-updated (the owner reads it, can't interject mid-item). Record
  wall-clock per item ([[feedback_track_estimated_vs_actual]]).

## Verification gate (after EVERY item; all green or revert the item)

```bash
~/.cargo/bin/cargo build -p ambition_sandbox
~/.cargo/bin/cargo test  -p ambition_sandbox --lib
~/.cargo/bin/cargo test  -p ambition_sandbox --test architecture_boundaries \
                                              --test scripted_gameplay \
                                              --test replay_fixture_regression
# plus any item-specific reachability tests noted on the item
```
For each NEW crate also `cargo build -p <new_crate> && cargo test -p <new_crate>`.
Never regenerate replay fixtures. `cargo fmt` before each commit.

---

## Backlog (prioritized; execute top-down, skip-and-note on block)

### Tier 1 — clean crate extractions (low risk, high structural value)

- **T1 · `ambition_time`** — extract `crate::time/` (WorldTime, ClockScale time-domains,
  camera-ease, feel; 1468 LOC, **zero outbound sandbox coupling**) into a new
  Layer-0 crate `crates/ambition_time/` exposing a `TimePlugin`. Sandbox depends on it;
  `crate::time` becomes a facade re-export. `ambition_platformer_runtime` (gravity/
  orientation, which currently mirror via `SimDt`) MAY then depend on `ambition_time`
  directly — but only if clean; otherwise leave the SimDt mirror. Handle the 2
  `crate::features::` refs in `camera_ease.rs`/`feel.rs` by keeping those couplings
  sandbox-side (move just the coupled fn back, or invert) — don't drag `features` into
  the crate. Gate: full + `movement_axis`.

- **T2 · `ambition_projectile`** — extract the **generic projectile physics primitive**
  (`projectile/{body,collision,spec}.rs` — brain-free) into a new crate. UNIFY: make
  `crate::enemy_projectile` (802 LOC, pirate volleys) and `crate::projectile` (player
  fireball) both consume the crate's `ProjectileBody`/collision rather than duplicating.
  The brain-coupled SPAWN (`projectile/systems.rs`, player-tick action messages) STAYS
  in sandbox as a thin consumer. Facade `crate::projectile` re-export. Gate: full +
  any projectile/collision tests. Fallback: if body/collision can't cleanly separate
  from systems.rs, extract just `spec` + `body` and note the remainder.

- **T3 · `ambition_music`** *(if clean)* — `crate::music/` (2076 LOC, only 1
  `crate::encounter` coupling) → a music-director crate. Verify its audio deps
  (`ambition_sfx`, `bevy_kira_audio`) are crate-clean first; invert the 1 encounter
  ref. If it drags in sandbox-specific roster/IDs, DEFER and note.

### Tier 2 — portal: make it crate-ready (the plan's flagship mechanic)

- **T4 · Portal P (render separation)** — split portal presentation out of portal core:
  `portal/presentation.rs` → a `portal_render` module/feature so portal *simulation*
  compiles without render-facing systems. Gate: full + `portal_bridge_reachability`,
  `portal_lab_usable`.
- **T5 · Portal Q (adapter removal)** — remove `ControlFrame` + `GroundItem` from portal
  core (`transit/plugin/presentation/messages/pickup.rs`) via portal input-intent
  messages + a generic transitable body/item component. Then portal can disable cleanly.
  Gate: full + portal reachability. Fallback: land whichever of the two couplings is
  clean; note the other.
- **T6 · `ambition_mechanics_portal`** *(only if T4+T5 land clean)* — extract portal →
  crate. Else defer to a future run.

### Tier 3 — structure & consistency

- **T7 · `mod.rs` normalization** — convert the remaining root sidecar pairs
  (`<mod>.rs` + `<mod>/` → `<mod>/mod.rs`) for the modules that stay in sandbox after
  the extractions above. ~20 pairs today (fewer after T1–T6 turn some into facades /
  remove them). Owner preference: the `mod.rs` style. Pure `git mv` + nothing else;
  one commit (or a few grouped). Gate: full (compile proves it).

### Tier 4 — enemy AI (structural prep only this run; feel changes deferred)

- **T8 · Brain/enemy-AI navigability pass** — the owner wants to improve enemy AI soon.
  `crate::brain` is 8.7k LOC / 12 files and central. SAFE autonomous scope: assess +
  *structurally* clarify (split oversized files, name the enemy-behavior seams, add a
  short `brain/README` or module docs mapping where AI behaviors live), with **zero
  behavior change** (gated by `scripted_gameplay` + `replay_fixture_regression` proving
  identical sim). Do NOT change AI behavior/feel — that needs the owner watching; tee
  up concrete improvement ideas in §Follow-ups instead. Fallback: if no clean
  structural win, write the assessment + idea list only.

### Tier 5 — close out

- **T9 · Guards + docs** — `architecture_boundaries`: assert each new crate's dependency
  direction (e.g. `ambition_time`/`ambition_projectile` must not depend on
  `ambition_sandbox`). Update this doc's §Progress + the memory index. Final
  `--features visible` build.

---

## Loose root-file organization (owner asked: where do these go?)

Classification of the remaining single-file root modules → most-elegant home. The
rule: **clear domain → its own `mod.rs` dir; lives-with-X → move into X; reusable +
uncoupled → extract; genuinely top-level/small/entry → stay.** Moves use
`git mv` + import-rewrite + the full gate, like the abilities slice.

**Consolidate into a domain module (root files → one `<domain>/` dir):**
- **`combat/`** ← `combat.rs`→`combat/mod.rs`, `combat_slots.rs`→`combat/slots.rs`
- **`actor/`** ← `actor.rs`→`actor/mod.rs`, `actor_control.rs`→`actor/control.rs`, `character_ai.rs`→`actor/ai.rs`
- **`items/`** ← `items.rs`→`items/mod.rs`, `item_pickup.rs`→`items/pickup.rs`, `shop.rs`→`items/shop.rs`, `inventory_persist.rs`→`items/persist.rs`

**Move into an existing home:**
- `lunex_kaleidoscope_app.rs` (4203 L — the biggest file in the crate; the cube menu host) → **`menu/kaleidoscope_app.rs`** (it IS menu code; huge nav win)
- `portal_pieces.rs` (portal Core invariant) → **`portal/pieces.rs`**
- `cutscene.rs` → **`presentation/cutscene/`**; `hud_overlay.rs` → **`presentation/hud.rs`**
- `save.rs` → **`persistence/save.rs`**

**Extract to a reusable crate:**
- `kinematic.rs` (417 L, generic kinematic body, **zero coupling**) → `ambition_platformer_runtime::kinematic`
- `falling_sand.rs` (1305 L, self-contained CA sim, only presentation/persistence/save) → its own `ambition_falling_sand` crate (a drop-in sim plugin) — strong "reusable systems for agents" candidate

**Stay at root (genuinely top-level / small / entry — moving would add noise):**
- `config.rs` (constants), `physics.rs` (facade shim), `debug_label.rs` (52 L generic),
  `headless.rs` (bin entry), `dialog_lint.rs` (dev lint — could → `dev/`),
  `interaction.rs` (generic interactable kit — candidate for a future `mechanics/`),
  `quest.rs` (→ `content/` later), `shrine.rs` (→ `world/` later), `ability_cooldown.rs`
  (→ `abilities/` — small, low priority)

Added as run slices: **T10** menu host move, **T11** combat/actor/items consolidation,
**T12** presentation+portal+persistence home moves, **T13** `kinematic`→runtime,
**T14** `ambition_falling_sand` crate.

## Progress (live)

| Item | Est | Actual | Commit | Status / notes |
|---|---|---|---|---|
| T1 ambition_time | 40m | ~75m | `bf729e1c` + `4104ba0a` | DONE (both steps) — **T1a** (`bf729e1c`): the entanglement was incidental. `time_scale` lived on the 2-field `SandboxSimState` god-struct (with `room_transition_cooldown`) but belongs to the TIME domain, so it moved to a new time-owned `crate::time::clock_state::ClockState { time_scale: f32 }`. Writers retargeted: the smoother, the suspended-frame zero, and reset/room-load/death/respawn (`reset_sandbox`, `load_room`, `death_respawn_player`, `safe_respawn_player`, `handle_player_damage_events`, `runtime::reset`). Readers retargeted: `refresh_world_time` (the WorldTime producer) + the dev/trace recorder (`build_frame`/`record_simulation_frame`/headless bin). `apply_room_transition_system` was at the 16-SystemParam limit, so the two reset resources are bundled in a small `RoomClock` SystemParam. **T1b** (`4104ba0a`): extracted the generic time vocabulary + producer into the reusable Layer-0 crate `crates/ambition_time/` — `WorldTime` + dt accessors (sim/wall/player/entity/dt_for), `ClockDomain`, `ClockState`, `ProperTimeScale`, `refresh_world_time`, and a documented `TimePlugin` (`app.add_plugins(TimePlugin)` → installs `ClockState`+`WorldTime`+the producer). The player-slot coupling was generalized to a crate-owned `ClockObserver(u8)` so the crate carries no game player type. **Stayed sandbox-side** (consume via the `crate::time` facade): the time-control POLICY (`Regime`/`Permission`/`ClockRequester`/`RegimePolicy`/`RequestedClockScale`/`apply_clock_scale_requests`/`emit_player_time_intent_system`/the feel-tuned smoother — all reference `PlayerSlot`/`PlayerBlinkState`/`SandboxFeelTuning`), `camera_ease`, `feel`, and `mirror_sim_dt_into_runtime` (bridge to the sibling runtime crate). Sandbox keeps its precise schedule wiring of `refresh_world_time` (not delegated to `TimePlugin`). Facade re-exports cover ~42 inbound sites with zero churn. New `architecture_boundaries_time_crate_is_extracted` guard (dep direction + content-freedom + TimePlugin + facades). **replay_fixture_regression: ZERO divergence after BOTH steps.** Gates: ambition_time build + 9 tests; sandbox build (incl. `--features visible`), lib 1428, architecture_boundaries 14, scripted_gameplay 3, movement_axis 2 — all green. The mid-run DEFERRED note below is now SUPERSEDED. |
| T2 projectile primitive → runtime | 50m | ~35m | `f315cf8e` | DONE — `projectile/{body,collision,spec}.rs` (brain-free physics primitive) `git mv`→`ambition_platformer_runtime/src/projectile/`; new `projectile/mod.rs` + lib/prelude re-exports; `crate::engine_core::…`→`ambition_engine_core::…`; the 1 `enemy_projectile` ref was a comment only (inverted/genericized). Grew the runtime crate per the Stage-16 lesson (NO new crate). Sandbox `projectile/mod.rs` keeps `mod systems/state/spawn/motion_input/visuals/diagnostics` (the brain-coupled player SPAWN) + `pub use ambition_platformer_runtime::projectile::*` facade → zero call-site churn; `crate::enemy_projectile` consumes the same primitive through the facade unchanged. Body/collision/spec inline tests rode along (34 runtime tests); spawn/QCF/integration tests (`engine_tests.rs`, `tests/`) stayed sandbox-side. serde added to runtime Cargo (ProjectileKind derive). All gates green. |
| T3 ambition_music | 35m | ~55m | `4215fc82` | DONE (decouple-in-place; crate split DEFERRED) — interrogated all 5 couplings. **encounter/content/rooms = INCIDENTAL** (the content track-map): inverted via a neutral seam. New `crate::music::state::MusicIntent { adaptive: Option<AdaptiveCueDirective>, simple_track_candidates: Vec<String> }` resource + a content-side `crate::music::intent::compute_music_intent` system that reads `EncounterRegistry`/`*MusicRequest`/`RoomMusicRequest`/`RadioStationState`/`SandboxDataSpec` + the catalog bindings, resolves the encounter-phase→cue-state directive (moved from the old `director/resolver.rs`, now deleted) and the simple-track priority list, and writes `MusicIntent`. The director (`drive_music_director` + `simple`/`adaptive`) now consumes ONLY `MusicIntent` + neutral catalog/assets/channels/audio-backend/volume — it imports **zero** `crate::encounter`/`crate::content`/`crate::rooms` and names no boss/encounter/track. `simple.rs` switched from `resolved_simple_track(room/radio/encounter/boss/data)` to a pure `resolved_simple_track(library, &[String])` (director owns only the "is it in the library?" pick). Scheduled `(compute_music_intent → drive_music_director).chain()` in the audio plugin; `MusicIntent` init there. **persistence = thin (kept):** the director still reads one `UserSettings.audio.effective_music()` volume float — left as a downward dep for the future crate to abstract. **audio = essential direction (kept):** `AudioLibrary`/`MusicChannel`/`MusicPlaybackState`/`switch_to_music_track` is the sandbox playback backend; fine to keep. **Crate split DEFERRED** (documented fallback): the director is not yet game-agnostic — it depends on the sandbox `crate::audio` track-by-id backend + `UserSettings`, and `catalog::builtin()`/`first_goblin`/`gains.rs` still bake `first_goblin` content. A clean `ambition_music` crate needs (a) a neutral audio-library/base-channel trait (or move to `ambition_sfx`), (b) a neutral volume resource, (c) the roster/gains content moved sandbox-side. New guard `architecture_boundaries_music_director_is_content_agnostic` (director forbids encounter/rooms/content; `intent.rs`+`tests.rs` are the content half). Gates: sandbox build clean, lib 1428, architecture_boundaries 15, scripted_gameplay 3, replay_fixture_regression 1 — all green; music unit tests 28 pass. |
| T4 portal P | 40m | ~25m | `47a3dcc8` | DONE — portal SIM now compiles/runs independently of portal RENDER. **Already separated** (prior work): the `portal_render` Cargo feature existed; ALL render-only system *registration* (`load_portal_gun_art`, `sync_portal_visuals`/`_body_pieces`/`_disorientation_indicator`/`_mode_indicator`, `portal_dev_toggle_system`, gravity-zone visuals) was already `#[cfg(feature = "portal_render")]` in `presentation/rendering/mod.rs`; `PortalPlugin` already delegated to a render-free `PortalSimulationPlugin`. **The remaining leak I closed:** `portal/presentation.rs` (Sprite/Text2d/Mesh-facing systems + the render-only `PortalAimHint`/`PortalGunArt`/`Portal*Indicator`/`PortalVisual`/`PortalBodyPiece` components) was compiled UNCONDITIONALLY and re-exported from `portal/mod.rs` regardless of feature, and the SIM plugin (`portal/plugin.rs`) `init_resource::<PortalAimHint>()` — a render-only resource — in the sim build. Fixed: gated `mod presentation` + its `pub use` block + the `portal_dev_toggle_system` re-export behind `#[cfg(feature = "portal_render")]`; moved the `PortalAimHint` init behind the feature; and gated the content input adapter's `PortalAimHint` import/param/write (`Option<ResMut>`, already a graceful no-op) behind the feature so `input`-without-`portal_render` builds. No sim component was reaching INTO a render component (the coupling was the reverse — render reads sim data + a render-owned aim hint), so ZERO simulation behavior changed. **Portal-sim-without-render COMPILES:** `--no-default-features --features "ldtk_runtime input portal"` (exit 0) — portal core is render-independent. Gates: default build clean, lib 1428, architecture_boundaries 15, scripted_gameplay 1, replay_fixture_regression 1 (**ZERO divergence**), portal_bridge_reachability 1, portal_lab_usable 3 — all green. Nothing deferred. |
| T5 portal Q | 50m | — | — | |
| T6 portal crate | 30m | — | — | conditional on T4+T5 |
| T7 mod.rs normalize | 40m | — | — | after extractions |
| T8 brain nav pass | 45m | ~30m | `8e640a6c` | DONE (docs-only) — added `brain/README.md`: the navigability map (policy=Brain vs capability=ActionSet seam; one-tick data-flow diagram snapshot→Brain→ActorControlFrame→ActionRequest/ActorActionMessage→sim EFFECTS; scheduling note pointing at `app/plugins.rs`; a "where each AI lives" table mapping player/NPC/enemy/brawler/boss → backend + behavior file + spawn site; file-by-file for all 12; key-type glossary). Appended 8 concrete **Enemy-AI improvement ideas** (flanking steering, crowd-rationed commits, behavioral telegraphs, reaction-delay/feint, ledge-aware retreat, skirmisher kiting band, aggro memory, boss phase reactions) each noting its plug-in site — teed up for the owner, NOT implemented. Added a 1-line README pointer to `brain/mod.rs` top doc. **No structural splits:** the three >1000 LOC files (`action_set.rs` 1513, `state_machine.rs` 1680, `boss_pattern.rs` 2169) each co-locate cfg/state pairs with their `tick_*` and share private helpers (`SignumOr`) + serde enums consumed by content RON; splitting risks `pub(crate)`/visibility + the replay gate for zero navigability gain (file-level `//!` docs + README already map them). Gates: build clean (doc-only), lib 1434 pass, architecture_boundaries 13, replay_fixture_regression zero-divergence, scripted_gameplay 3 — all green. |
| T9 guards + docs | 25m | — | — | last |
| T10 kaleidoscope→menu | — | ~10m | `c9352d08` | DONE — `lunex_kaleidoscope_app.rs`→`menu/kaleidoscope_app.rs` (8 sites) |
| T11 combat/actor/items consolidation | — | ~35m | `b3df018f` | DONE — 9 root files→`combat/`,`actor/`,`items/` mod.rs dirs; `crate::shop` facade kept for Yarn bindings; 3 architecture_boundaries path guards retargeted |
| T12 presentation/portal/persistence home moves | — | ~30m | `c9352d08` | DONE — `portal_pieces.rs`→`portal/pieces.rs`, `cutscene.rs`→`presentation/cutscene/script.rs` (existing player→`cutscene/mod.rs`), `hud_overlay.rs`→`presentation/hud.rs`, `save.rs`→`persistence/save_data.rs`; `crate::save` facade kept; time-guard cutscene allowlist path fixed. cutscene/save targets suffixed (collision-free) since the doc's literal paths already host distinct modules |
| T13 kinematic→runtime | — | ~15m | `20cc9e60` | DONE — `kinematic.rs` (417 LOC) → `ambition_platformer_runtime/src/kinematic.rs`; `crate::engine_core::…`→`ambition_engine_core::…` (engine_core dep already present); `KinematicBody/KinematicTuning/KinematicInputs/step_kinematic` added to runtime `prelude`; sandbox keeps `pub use ambition_platformer_runtime::kinematic` facade (zero call-site churn). All gates green incl. architecture_boundaries |

**Root `.rs` count: 29 → 15** after T10/T11/T12 (removed the 14 relocated files;
remaining roots are the documented "stay at root" set plus `kinematic.rs` /
`falling_sand.rs`, deferred to T13 / T14, and the `lib.rs` / `main.rs` /
`headless.rs` entries).

## Entanglement interrogation (owner: "is the coupling for a good reason? penalty to fix?")

| Entangled | Why | Essential/incidental | Penalty | Verdict |
|---|---|---|---|---|
| time→`SandboxSimState` | `WorldTime` producer reads `time_scale`, stored in the `SandboxSimState` god-struct | **Incidental** (`time_scale` belongs to time, not a 2-field grab-bag; per-player ClockDomain is real but lives in the *policy*) | **LOW** — move 1 `f32` out (6 writes + few reads); identical-sim gate proves dt unchanged | **DONE** — DECOUPLED (T1a) + crate EXTRACTED (T1b), zero replay divergence |
| music→`encounter`/`content`/`rooms` | director picked cue/track by reading encounter phase + room/content track ids | **Incidental** (generic director vs. content track-map) | **MODERATE** — invert to a neutral `MusicIntent` pushed by a content-side `compute_music_intent` | **DONE** — DECOUPLED in-place (T3): director imports zero encounter/content/rooms, names no boss/track; consumes only `MusicIntent`. Crate split DEFERRED (director still on sandbox `crate::audio` backend + `UserSettings`; roster/gains still carry `first_goblin` content) |
| music→`audio`/`persistence` | director uses the playback backend + one volume float | **Essential direction** (audio) / **thin** (one `UserSettings` float) | LOW | KEPT — legitimate downward deps; a future crate would abstract the audio-library/base-channel + volume into neutral types |
| falling_sand→`rooms`/`config` | CA sim woven with room chunk-loading | **Partly essential** (room bounds genuinely needed) | **HIGH** (1305 LOC interwoven) | document, defer |
| brain centrality | used by actors/content/projectile | **Essential by design** (universal-brain seam) | extracting it fights the architecture | keep central; get named behaviors out instead |

## Coupling findings + deferrals (recorded mid-run 2026-06-08)

A full-coupling recheck (the first pass under-counted — it missed `SandboxSimState`/
`player`/content refs) reclassified three "extraction" candidates as NOT cleanly
extractable tonight. Deferred with the seam each needs (design tasks, not mechanical
moves — doing them blind overnight would risk core regressions):
- **T1 `ambition_time` — SUPERSEDED (now DONE, see Progress table).** The earlier
  recheck called this deferred, but the coupling was incidental: `time_scale` was just
  a field on the `SandboxSimState` god-struct, not a real time↔sim entanglement. T1a
  moved it to a time-owned `ClockState`; T1b then extracted the generic vocabulary +
  producer into `ambition_time` (the `ClockObserver(u8)` seam decoupled the player-slot
  type; the policy/camera-ease/feel stayed sandbox-side via the facade). Zero replay
  divergence both steps.
- **T3 `ambition_music` — SUPERSEDED (decouple-in-place DONE; crate split still deferred).**
  The earlier note called this fully deferred; the encounter/content/rooms coupling was
  incidental (the content track-map) and is now inverted via the neutral `MusicIntent`
  seam (see Progress T3). The director imports none of encounter/content/rooms. The
  *crate* split remains deferred: the director still rides the sandbox `crate::audio`
  track-by-id backend + `UserSettings`, and `catalog::builtin()`/`first_goblin`/`gains.rs`
  still carry `first_goblin` content — a clean crate needs those abstracted/moved first.
- **T14 `ambition_falling_sand` — DEFERRED.** Heavy coupling (config 15, rooms 10, …) —
  needs room/config inversion first.

CLEAN extractions that landed: **T13 kinematic** + **T2 projectile primitive** →
`ambition_platformer_runtime` (both `engine_core`-only).

## Follow-ups (deferred — need owner / later run)
- Time/music/falling-sand crate extractions — need the generic seams above.
- Enemy-AI *behavior* improvements (feel — owner must watch).
- Phase 3: promote `ambition_content` to a crate, retarget assets, rename
  `ambition_sandbox` → shell (`ambition_app`/`ambition_game`).
- Duplicate-subsystem sweep beyond projectile.
