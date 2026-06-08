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
| T1 ambition_time | 40m | — | — | next |
| T2 projectile primitive → runtime | 50m | ~35m | `f315cf8e` | DONE — `projectile/{body,collision,spec}.rs` (brain-free physics primitive) `git mv`→`ambition_platformer_runtime/src/projectile/`; new `projectile/mod.rs` + lib/prelude re-exports; `crate::engine_core::…`→`ambition_engine_core::…`; the 1 `enemy_projectile` ref was a comment only (inverted/genericized). Grew the runtime crate per the Stage-16 lesson (NO new crate). Sandbox `projectile/mod.rs` keeps `mod systems/state/spawn/motion_input/visuals/diagnostics` (the brain-coupled player SPAWN) + `pub use ambition_platformer_runtime::projectile::*` facade → zero call-site churn; `crate::enemy_projectile` consumes the same primitive through the facade unchanged. Body/collision/spec inline tests rode along (34 runtime tests); spawn/QCF/integration tests (`engine_tests.rs`, `tests/`) stayed sandbox-side. serde added to runtime Cargo (ProjectileKind derive). All gates green. |
| T3 ambition_music | 35m | — | — | conditional |
| T4 portal P | 40m | — | — | |
| T5 portal Q | 50m | — | — | |
| T6 portal crate | 30m | — | — | conditional on T4+T5 |
| T7 mod.rs normalize | 40m | — | — | after extractions |
| T8 brain nav pass | 45m | — | — | structural only |
| T9 guards + docs | 25m | — | — | last |
| T10 kaleidoscope→menu | — | ~10m | `c9352d08` | DONE — `lunex_kaleidoscope_app.rs`→`menu/kaleidoscope_app.rs` (8 sites) |
| T11 combat/actor/items consolidation | — | ~35m | `b3df018f` | DONE — 9 root files→`combat/`,`actor/`,`items/` mod.rs dirs; `crate::shop` facade kept for Yarn bindings; 3 architecture_boundaries path guards retargeted |
| T12 presentation/portal/persistence home moves | — | ~30m | `c9352d08` | DONE — `portal_pieces.rs`→`portal/pieces.rs`, `cutscene.rs`→`presentation/cutscene/script.rs` (existing player→`cutscene/mod.rs`), `hud_overlay.rs`→`presentation/hud.rs`, `save.rs`→`persistence/save_data.rs`; `crate::save` facade kept; time-guard cutscene allowlist path fixed. cutscene/save targets suffixed (collision-free) since the doc's literal paths already host distinct modules |
| T13 kinematic→runtime | — | ~15m | `20cc9e60` | DONE — `kinematic.rs` (417 LOC) → `ambition_platformer_runtime/src/kinematic.rs`; `crate::engine_core::…`→`ambition_engine_core::…` (engine_core dep already present); `KinematicBody/KinematicTuning/KinematicInputs/step_kinematic` added to runtime `prelude`; sandbox keeps `pub use ambition_platformer_runtime::kinematic` facade (zero call-site churn). All gates green incl. architecture_boundaries |

**Root `.rs` count: 29 → 15** after T10/T11/T12 (removed the 14 relocated files;
remaining roots are the documented "stay at root" set plus `kinematic.rs` /
`falling_sand.rs`, deferred to T13 / T14, and the `lib.rs` / `main.rs` /
`headless.rs` entries).

## Coupling findings + deferrals (recorded mid-run 2026-06-08)

A full-coupling recheck (the first pass under-counted — it missed `SandboxSimState`/
`player`/content refs) reclassified three "extraction" candidates as NOT cleanly
extractable tonight. Deferred with the seam each needs (design tasks, not mechanical
moves — doing them blind overnight would risk core regressions):
- **T1 `ambition_time` — DEFERRED.** `WorldTime` couples to `crate::SandboxSimState`
  (central sim state, reads `time_scale`) + player clocks. Needs a generic time-source
  seam (crate owns `WorldTime`+`ProperTimeScale`+dt math; sandbox feeds via a producer).
- **T3 `ambition_music` — DEFERRED.** Couples to `crate::encounter`+`crate::content`
  (track-per-boss) — named-content entanglement; needs a generic director vs. roster split.
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
