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

## Progress (live)

| Item | Est | Actual | Commit | Status / notes |
|---|---|---|---|---|
| T1 ambition_time | 40m | — | — | next |
| T2 ambition_projectile | 50m | — | — | |
| T3 ambition_music | 35m | — | — | conditional |
| T4 portal P | 40m | — | — | |
| T5 portal Q | 50m | — | — | |
| T6 portal crate | 30m | — | — | conditional on T4+T5 |
| T7 mod.rs normalize | 40m | — | — | after extractions |
| T8 brain nav pass | 45m | — | — | structural only |
| T9 guards + docs | 25m | — | — | last |

## Follow-ups (deferred — need owner / later run)
- Enemy-AI *behavior* improvements (feel — owner must watch).
- Phase 3: promote `ambition_content` to a crate, retarget assets, rename
  `ambition_sandbox` → shell (`ambition_app`/`ambition_game`).
- Duplicate-subsystem sweep beyond projectile.
