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

## Honest takeaway for the next session
The only **big + mechanically-easy** win was menu (done). Everything else is either:
- a DOWN-extraction blocked by upward machinery deps (mechanics, world, presentation,
  brain, boss_encounter), or
- a carve-the-state-from-the-systems job (dev remainder, features actor core).

Pick the next target by the *measured outward-dep count*, not the line size or the
content-guard status. The cheapest remaining real wins are probably **brain → crate**
(medium; the named boss-attack variants are the only knot) and the **dev carve**
(state/systems split, well-understood pattern). The presentation B3 boss-asset map
(sketch in `dev/journals/code_smells.md`) remains but needs Jon's eyes (visual).
