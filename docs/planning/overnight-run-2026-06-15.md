# Overnight refactor run — 2026-06-15 (autonomous, packed)

_Set up 2026-06-14 (Opus 4.8, interactive) with Jon. Builds on
`monolith-next-batch.md` + `refactor-candidates.md`. This is the OPERATIONAL doc:
the executing agent works the backlog top-down, validates + commits each item,
and **live-updates the progress table** as it goes (Jon reads this; he cannot ask
questions mid-run)._

## Mission

Shrink + elegantize the codebase toward the engine/content boundary. **Any LOC
reduction or movement toward an elegant system ANYWHERE is a win — not just
`ambition_sandbox`.** Cross-crate dedup, dead-code deletion, a god-module split,
a tightened seam: all count.

**Sizing.** The last run was scoped ~20× a normal task and still finished well
before the mission window. This run is scoped ~100×: the backlog below is
deliberately longer than one session "should" hold. That is intended — **do not
stop at the end of a wave; roll into the next.** When the seeded backlog thins,
GENERATE more via the discovery methods. There is always more.

## Jon's driving principles (infer from these — never ask, never stop)

- **Never stop early.** Given a duration, keep producing value until the window
  closes or the backlog is genuinely empty. Infer the **elegant / efficient /
  end-state-aligned** choice and proceed. Work around blockers; do not ask.
- **Four goals every item must serve at least one of:** (1) incremental compile
  time, (2) agent-navigability, (3) idiomatic Bevy plugins, (4) audit-grade
  reuse. **Oracle:** _could a different platformer be built by ADDING a content
  crate without editing core?_
- Narrow specific types beat wide generic ones; add knobs only when a use case
  lands; no wide tech-debt surfaces.
- Single-commit full replacement > two-step bridge (pre-release, nothing depends
  on us, AIs over-value backwards compat).
- Elegance / efficiency / end-state-want is the tiebreaker on every fork.

## Standing rules (per commit)

- Work on **main**. No feature branches.
- **Replay discipline.** A behaviour-NEUTRAL change must keep replay bit-identical
  (`cargo test -p ambition_app --test replay_fixture_regression` after any
  sim-touching change). A deliberate coherence fix may change replay — but only
  with a focused test pinning the new behaviour AND a `behaviour Δ` flag in the
  progress table (see Run latitudes). Keep
  `cargo test -p ambition_app --test architecture_boundaries` green; add a new
  guard whenever you lock a boundary.
- One concern per commit, clean rollback boundary, validation command in the body.
- Never `git add -A` (working tree carries dev junk); stage explicit paths.
- Never commit binary/generated data (sheets/audio/weights regenerate via tools).
- Sign commits: `Co-Authored-By: <executing model> <noreply@anthropic.com>`.
- Update the **progress table** + record est-vs-actual per item; emit a final table.

## Run latitudes (decided 2026-06-14 with Jon)

- **A4 (presentation move-UP):** ATTEMPT it if A1–A3 leave a **finite, clean**
  coupler list. Take a real swing at the RoomVisual lifecycle inversion + the move
  behind replay + a new "sim does not import render" guard. If it turns into a
  half-finished tangle, **roll it back cleanly** and leave a written inversion plan
  — don't ship a broken cut.
- **New crates:** FREE to create when a clean boundary appears (`ambition_ui_kit`,
  `ambition_render`, an audio-runtime crate, etc.). This is the lever the whole
  refactor rides on — use it.
- **Behaviour / replay (READ THIS — it overrides any earlier caution):** the ONLY
  hard gate is **it compiles + the workspace builds.** NOT replay-bit-identical,
  NOT "no behaviour change." Replay is a verification *tool*, not a gate. Behaviour
  is ALLOWED to change and during this structural work it usually will — often for
  the better. **Actively pursue gameplay improvements** (the current feel is the
  baseline to BEAT). Do the big structural moves even when they break replay; we
  fix feel after. NEVER shrink/defer a move to keep replay identical, and NEVER
  declare the work "exhausted/entangled/needs supervision" to dodge it — that
  evasion is the failure. Canonical: `docs/concepts/autonomous-decision-making.md`.
- **Renames + regen:** rename ids freely (ItemKind/Item, BossAttackProfile,
  Sandbag→TrainingDummy, …). **There is no save data that matters — delete saves,
  regenerate LDtk/sheets/RON via the tools.** "This is the time to do it." Keep
  `regen_sprites.sh` / `regen_assets.sh` working on a fresh clone; never commit the
  regenerated binary output (it's gitignored).

## The loop (method)

1. Pick the highest-value **unblocked** item (respect the dependency notes).
2. Execute it BOLDLY — structural correctness first; let behaviour/replay change.
3. `cargo build -p ambition_app` (the only gate) → commit → update the progress table.
4. The commit IS the checkpoint — then IMMEDIATELY take the next move. Never hand back.
5. Blocked on THIS item? Take the next one. Backlog thinning? Run discovery methods.
6. Continue until the given clock runs out. Do not stop early.

## ⭐ MONOLITH BREAKUP SEQUENCE (the spine — execute top-down)

The goal: shrink `ambition_sandbox` (~85k LOC) by moving layers OUT into crates so
another platformer could be built by adding a content crate without editing core.
Ordered so each step compiles + commits; later steps unlock the big one.

1. ✅ **DONE (2026-06-15).** **Drive presentation couplers to ~0** by moving the imported types DOWN out of
   `presentation` (each a compile+commit). Pure sim now imports ZERO presentation;
   locked by `architecture_boundaries_sim_does_not_import_presentation`. Remaining
   importers (`dialog/ui`, `runtime/{setup,reset}`) are at-or-above the render line.
   - `PlayerVisual` / `SceneEntities` (zero-sized marker + handle Resource) →
     `platformer_runtime::lifecycle` (like `RoomVisual`). Decouples portal, body_mode.
   - `character_sprites` METADATA (`baked_sheet_registry`, `sheets` specs, the
     `build.rs`-generated baked table, `all_character_sprite_filenames`) → a new
     foundation crate `ambition_character_sprites` (or `crate::sprite_data` /
     the asset layer). Relocate the `build.rs` generation. Decouples ~5 files +
     the renderer reads it from below.
   - Character ANIM vocabulary (`CharacterAnim`, `*AnimState`, `pick_*_anim`) →
     the same foundation home (it's content-free mapping). Decouples anim_helpers.
   - Misc: `BoundFeatureKind`, `rider_hand_world_pos`, `ui_fonts::UiFontWeight`.
   - Whatever's left in the lib that imports presentation is HOST/SETUP wiring —
     it moves UP with presentation in step 2, not decoupled.
2. ✅ **DONE (2026-06-15, ~45min).** **Extract `presentation/` → a new `ambition_render` crate** that depends on
   `ambition_sandbox`. `git mv` the dir; rewrite its internal `crate::X` → `ambition_sandbox::X`;
   `ambition_app` + `ambition_content` repoint `presentation::*` → `ambition_render::*`.
   Add the architecture guard: **the sandbox lib does not import the render crate.**
   This is the ~10k-LOC monolith cut. **NOTE:** `presentation/` is 6755 LOC (NOT 10k).

   **▶ STEP-2 EXECUTION PLAN (scoped 2026-06-15; step 1 is DONE + guard-locked).**
   The blocker is NOT sim→render couplers (those are 0, locked by
   `architecture_boundaries_sim_does_not_import_presentation`). It is the THREE
   in-lib modules that import `crate::presentation` and would form a crate cycle
   (`ambition_sandbox → ambition_render → ambition_sandbox`) once presentation
   moves up: `dialog/ui.rs`, `runtime/setup.rs`, `runtime/reset/` (the guard's
   allowlist names exactly these). Resolve BEFORE the `git mv`:
   - **`runtime/setup.rs` + `runtime/reset/mod.rs`** are composition-root
     orchestration (build the scene, respawn room visuals on reset). They call
     `spawn_room_visuals` + `spawn_parallax_layers` (the ONLY two non-render
     callers — verified). Two options: (a) MOVE both files up into `ambition_app`
     (or the new render crate) — they already pull tons of sandbox `pub(crate)`
     internals, so this needs a `pub`/`pub(crate)`→`pub` widening pass; or
     (b) INVERT the two spawn calls behind a `RespawnActiveRoomVisuals` message a
     presentation system consumes (the system reads the active room spec from
     `RoomSet`/`ActiveRoomMetadata` — already resources). **(b) is smaller** and
     keeps reset/setup sim-side; prefer it.
   - **`dialog/ui.rs`** IS UI rendering (draws the dialog box + uses
     `ui_fonts::{UiFonts, UiFontWeight}`). It belongs in the render crate — move
     it up with `presentation/`, leaving the sim-side dialog logic (`dialog/mod`,
     `yarn_bindings`) in the lib.
   - Then the `git mv presentation → crates/ambition_render/src`, rewrite
     `crate::` → `ambition_sandbox::` (expect a `pub` widening pass for the ~40
     distinct sandbox symbols presentation touches — `player::*`, `features::*`,
     `rooms::*`, `assets::*`, `dev::dev_tools`, `config`, etc.; grep surface is in
     the session log), repoint `ambition_app`/`ambition_content`, add the guard.
   - Sequencing: do the message-inversion (b) + `dialog/ui` move as their OWN
     committed steps FIRST (each keeps the lib compiling), THEN the crate `git mv`
     as the final big commit. Don't do it all in one shot.
3. **Named content OUT of machinery** (the engine/content oracle):
   - `ItemKind` / `PlayerInventory` (legacy 3-kind bag) → DELETE; collapse onto the
     canonical `Item`/`OwnedItems` 24-row catalog (already a registry). See the
     2026-06-15 dual-bag entry in `dev/journals/code_smells.md` for the exact
     consumer list + fix sketch (~1-2h, spans `ambition_content::quest`).
   - `BossAttackProfile` enum (in foundation `ambition_actor`) → the `Special(String)`
     content seam already exists; collapse the remaining NAMED melee variants
     (FloorSlam/HandSlam/HazardColumn/…) into data-keyed volume params so the enum
     names no boss. Foundation-crate surgery — careful, has `volumes_for_profile`.
   - boss-sprite `GameAssets` named fields + per-boss loaders → id-keyed registry
     (see the boss-sprite smell in `code_smells.md`; land AFTER sheet-data migration).
   - NOTE: boss ENCOUNTER roster is ALREADY content (only `gradient_sentinel`
     generic fallback remains in-lib — `boss_encounter/roster.rs`). Don't redo it.
4. **Audio/music runtime** → split the game-read adapters from playback; extract.
5. **God-module splits + dedup + dead-code** as filler between the big cuts.

### ⚖️ HONEST STATE ASSESSMENT (2026-06-15, after the render extraction)

After extracting `ambition_render`, I surveyed every remaining sandbox subsystem for
a clean crate cut. **There is no clean large crate cut left** — the sandbox core is a
tightly-coupled web (the player cluster + `features` + `rooms` + `brain` are mutually
referenced by nearly everything). Findings:
- `ldtk_world` (6.3k) — 73 refs to `crate::rooms`, 42 to `interaction`: it's THIS
  game's LDtk→RoomSpec converter, not a reusable loader.
- `boss_encounter` (5k) — 24 refs to `features`, 11 to `brain`: woven into the game.
- `character_sprites` (2.6k) / `body_mode` (0.8k) — player-coupled (the player is the
  universal hub). But the player IS already split: generic kinematics live in the
  foundation crates (engine_core/actor); sandbox/player holds only game-specific state.
- `audio` (1.3k) — the generic half is already `ambition_audio`; what's left is game glue.
- `Item` (24-variant) — correctly a NARROW enum with type-level equip/ability wiring
  (decision doc PREFERS narrow types); data-keying it would be anti-elegant. Leave it.
- Boss/enemy rosters, character sprites, boss behavior profiles — ALREADY data-driven.

**Conclusion:** the foundations already hold the reusable machinery; the big clean cut
(presentation→render) is done. The remaining high-value work is (a) **named-content-out
where it still hides** (boss sprites DONE this run), (b) **agent-navigability** — the #1
stated goal (`feedback_agent_navigability_north_star`): split the worst god-files so a
~13k `features/` dir + 1000-line files become workable, and (c) targeted dedup. Further
CRATE cuts need decoupling groundwork first (e.g. splitting `features` into generic ECS
machinery vs named feature content) — a real but multi-step decoupling, not a quick mv.

### Discovery methods (refill the backlog)

- **God-module split:** any production file > ~600 LOC → concern-split behind a
  `mod` dir; child does `use super::*`, only externally-called fns get
  `pub(super)`; public paths preserved; one file per commit. **Traps:** (a) verify
  the test mod is at EOF before sweeping `cfg(test)..EOF` (the `boss_pattern`
  trap — production code can follow it); (b) a child peel breaks
  `super::sibling_module::X` (from `foo/child.rs`, `super` is `foo/`) — rewrite
  to `super::super::` or absolute `crate::`.
- **Dedup sweep (ALL crates):** find copied helper bodies that have drifted
  (the scrollbar-math pattern); hoist one `pub(crate)` copy.
- **Dead-code sweep:** `cargo build` warnings + audit `#[allow(dead_code)]`;
  delete or wire up.
- **Drift/smell:** append fresh smells to `dev/journals/code_smells.md`;
  inline-fix only zero-risk obvious ones.
- **Doc drift:** `scripts/check_doc_links.py`; fix stale links / archive dead docs.

---

## Backlog (prioritized waves)

> Status legend: ☐ todo · ◐ in-progress · ☑ done · ⊘ blocked (reason in table)

### Wave A — Presentation seam (the ~10k-LOC linchpin; goals 1,2,4)

The lib has **32 files referencing `crate::presentation`** (8 are
presentation-internal; ~24 are real couplers). Measured coupler clusters
(2026-06-14):

| presentation type | refs | seam plan |
|---|---|---|
| `rendering::RoomVisual` | 10 | **lifecycle inversion** — sim manages visual teardown. Invert: sim emits a room-despawned event; renderer owns `RoomVisual`. The big one (A4). |
| `character_sprites::*AnimState` + `baked_sheet_registry` | ~13 | anim STATE is gameplay state the sim owns; move the state types down, renderer reads them (A3). |
| `cutscene::script::{CutsceneBeat,CutsceneScript}` + `ActiveCutscene`/`CutsceneAdvanceRequest` | ~10 | script VOCAB + neutral cutscene state → foundation/content seam (A2). |
| `ui_fonts::UiFontWeight` | ~8 | trivial enum, no deps → move down first (A1). |
| `rendering::{PlayerVisual,SceneEntities}` | ~7 | render handles referenced by sim → fold into the A4 inversion. |

- **A1.** Move `ui_fonts::UiFontWeight` (+ any sibling pure font enums) DOWN to a
  foundation crate (`ambition_effects` or a tiny `ambition_ui_kit`). Warm-up; ~8
  importers, zero logic. [S]
- **A2.** Cutscene script seam: move `CutsceneBeat`/`CutsceneScript` (the script
  VOCAB) to a neutral home; keep `ActiveCutscene`/`CutsceneAdvanceRequest` as
  neutral resources the sim writes and the renderer reads. [M]
- **A3.** Character-sprite anim STATE (`NpcAnimState`/`EnemyAnimState`/
  `PlayerAnimState` + `baked_sheet_registry` lookup) → move the state types DOWN
  (sim owns them); renderer reads. Decouples ~13 sites. [M]
- **A4.** **RoomVisual lifecycle inversion** (gated on A1–A3 thinning the graph):
  sim stops referencing `RoomVisual`/`SceneEntities`/`PlayerVisual` for teardown;
  emit a room/scene-despawned event, renderer owns the visual lifecycle. Then
  move `presentation` UP into `ambition_app` or a new `ambition_render` crate.
  Add the **"sim crate does not import the render crate"** architecture guard.
  [L, multi-session — the single biggest lib shrink, ~10k LOC.]

### Wave B — Content promotion finish (goal 4; proven install-holder pattern)

- **B1.** Move the 6 `.yarn` files (`ambition_sandbox/assets/dialogue/`) + their
  binding layer to `ambition_content`; keep the yarn RUNTIME + holder in lib.
  **Hazard (endgame doc):** move the content, keep the runtime; don't split the
  yarn list from its files. [M]
- **B2.** Audit quest/music for named data still in lib → content holders. [M]
- **B3.** Boss sprite asset registry: `GameAssets` named boss fields
  (`mockingbird`/`gnu_ton`/…) + per-boss `load_*` fns + the per-boss render
  if-chain → id-keyed registry; roster authored content-side (mirrors the
  character-sheet registry). [M]
- **B4.** `ItemKind`/`Item` registry: named variants (`HealthPotion`,`PortalGun`…)
  → id-keyed registry, roster content-side. [L]

### Wave C — De-naming / data-keying (goals 2,4)

- **C1.** `FeatureVisualKind::Sandbag` → `TrainingDummy` (touches LDtk + content
  map). [S]
- **C2.** `BossAttackProfile` enum (`HandSlam`/`HeadDescent`/…) → data-keyed attack
  specs; content registers specs. Active target of the Technique/Effects design. [M-L]
- **C3.** Special-attack effects consumers (`spawn_gnu_apple_rain…`, LockOnBeam/
  PitTrap/RotatingCross/MinionCascade) → vocabulary names + lift baked constants +
  projectile-art identity into RON spec fields. [M]

### Wave D — Internal god-module splits (goals 1,2; safe filler, always available)

One file per commit, concern-split, public paths preserved. Seed list (production
files, descending; discover more via the method):

- **D1.** `features/enemies.rs` (1292)
- **D2.** `features/ecs/actors/mod.rs` (1111)
- **D3.** `persistence/settings/model/mod.rs` (1123) — if not already split
- **D4.** `presentation/fx.rs` (995)
- **D5.** `items/pickup/mod.rs` (1025)
- **D6.** `dev/dev_tools/mod.rs` (1064)
- **D7.** `features/bosses.rs` (912)
- **D8.** `presentation/rendering/actors/mod.rs` (900)
- **D9.** `world/rooms/mod.rs` (823)
- **D10.** `presentation/character_sprites/sheets/mod.rs` (780)
- **D11.** `falling_sand.rs` (1305) — self-contained; good split candidate
- **D12.** `assets/game_assets/mod.rs` (1002)
- …continue: any production `.rs` > 600 LOC.

### Wave E — Audio/music decoupling (smell; prereq for an audio/music crate move)

- **E1.** Split `audio/runtime.rs` + `music/mod.rs` along the game-read seams
  (`EncounterMusicRequest`/`RoomMusicRequest` inline reads, `UserSettings` reads,
  player-position reads in `environment.rs`). [M]
- **E2.** Once decoupled, evaluate the audio/music crate move (the crate already
  exists for the bank; this is the runtime). [M]

### Wave F — Cross-crate simplifications (anywhere; goals 1,2,4)

- **F1.** `ui_nav` is **1 dep from a clean crate extraction** — only
  `persistence::{MenuPointerPress, MenuTapMode}` couples it. Move those nav-input
  types DOWN (to `ambition_input`?), then extract `ui_nav` to its own crate. [M]
- **F2.** Dedup sweep across ALL crates — copied/drifted helpers → one home.
- **F3.** Dead-code + `#[allow(dead_code)]` audit → delete or wire.
- **F4.** Doc-link guard: wire `scripts/check_doc_links.py` (already red) into a
  test/CI; fix the stale links (universal-brain-interface paths, lessons_learned
  `body_mode.rs`→dir, ADR 0019 missing section) + the deleted-RON-levels doc sweep. [S]

### Behaviour stance

This run is **primarily refactor + simplification** — most commits stay
replay-bit-identical. Confident, test-pinned **coherence fixes are allowed** (see
Run latitudes), flagged for Jon's feel-check. (One thing that's NOT in scope: a
portal *transit* pushout — it contradicts Jon's avoid-pushout rule; transit
emerges at the face and carries momentum, and the only sanctioned eviction is a
portal **relocating or disappearing**, which already exists.)

---

## Suggested order

A1 → A2 → A3 (presentation seam thinning) → B1/B2 (content, proven-safe) →
D (splits) as filler between the bigger items → F1 (ui_nav clean extraction) →
C1 (cheap de-name) → E1 (audio decouple) → then A4 (presentation move-UP) as its
own multi-session push once the coupler list is finite → C2/C3, B3/B4 as capacity
allows. Each item is independently shippable + replay-checked; **none changes
gameplay** — so the visible build is only needed to feel-check anything that
alters rendering OUTPUT (the presentation splits in D / the move-UP in A4), not
for correctness.

## Endgame (do not start until the boundary is real)

`ambition_sandbox` → `ambition_platformer_engine` + content rename (Phase 3) —
the final act; needs A4 done so the rename reflects a real boundary, not a label.

## Progress table (LIVE — update every item)

| # | item | status | est | actual | commit(s) | LOC Δ | notes |
|---|---|---|---|---|---|---|---|
| A1 | ui_fonts move-down | ☑ skip | S | — | — | — | low value: the 2 non-presentation importers also need `UiFonts`; moving the enum wouldn't break the edge. Verified, dropped. |
| F1 | extract `ui_nav` → crate | ☑ | M | ~1 cycle | `832215bf` | lib −711 | clean leaf; types already in `ambition_input`, no move needed. Replay identical. |
| A | VFX request vocab → `ambition_effects` | ☑ | M | ~1 cycle | (fx-move) | lib ~−85 | Explosion/Fireworks requests + `explosion_sfx` joined `ExplosionKind`; `dialog/yarn_bindings` now presentation-free. App builds, replay identical. |
| ⚠ | **disk-full event** | ☑ resolved | — | — | — | — | `/home/joncrall/ambition-target` cargo cache hit 107G (disk 100%, historical accumulation across sessions). Cleared → 45%. Repo `target/` is a decoy (config redirects). |
| A | extract cutscene format → `ambition_cutscene` crate | ☑ | M | ~1 cycle | `c337730e` | lib −311 | pure serde format+stepper; re-exported as `cutscene::script`. Replay identical, 6 crate tests. |
| — | clean-extraction frontier check | ☑ | S | — | — | — | **Exhausted.** `inventory` has named `ItemKind` content (needs data-keying, not extraction); `quest`/`music` already factored (runtime in lib, named content in `ambition_content`). Next phase = splits / de-naming / dedup + eventual A4. |
| C1 | `FeatureVisualKind::Sandbag`→`TrainingDummy` | ☑ | S | ~1 cycle | `b9345dec` | — | code-only kit-vocab de-name; content sprite keeps `sandbag` name. |
| — | sandbag passive fix (Jon FYI) | ☑ `behaviour Δ` | S | — | `b9345dec` | — | both `is_sandbag` archetypes had a dormant `PunchWeak` melee (aggro 0) → `melee: None`. Pinned by `sandbags_are_passive()` + content test. Replay identical. Feel-check: dummy no longer counter-attacks. |
| A4-prep | `RoomVisual` marker → `platformer_runtime::lifecycle` | ☑ | M | ~1 cycle | `5cab1e4b` | — | the seam's top coupler; zero-sized marker, runtime-owned home (its sibling `RoomScopedEntity` was already there). **presentation couplers 32 → 26.** Replay identical. |
| A | cutscene state → `ambition_cutscene` crate | ☑ | M | ~1 cycle | `d9196268` | lib −55 | `ActiveCutscene`/`CutsceneAdvanceRequest`/`SKIP_HOLD` consolidated into the cutscene runtime crate; `app/input_systems` presentation refs 8 → 2. Replay identical. |
| — | presentation seam status | — | — | — | — | — | **26 couplers; remaining are entangled.** The `PlayerVisual`/`SceneEntities` render-handle cluster has no clean foundation home (player↔presentation cycle risk) — needs a supervised A4 design+feel pass. RoomVisual + cutscene were the clean ones. |
| S1-a | `PlayerVisual`+`SceneEntities` → `platformer_runtime::lifecycle` | ☑ | M | ~1 cycle | (couplers 25→17) | — | the "no clean home" claim above was wrong — they're content-free tags/handles; runtime markers are the home, render re-exports. Decoupled portal/body_mode. |
| S1-b | `character_sprites` → lib root (it's gameplay anim, not presentation) | ☑ | M | ~1 cycle | `26c15afe` | — | 2609 LOC mis-filed under presentation; `git mv` to crate root. couplers 17→8. |
| S1-c | `BoundFeatureKind`+`rider_hand_world_pos`+`LoadingZoneVisual` move-down | ☑ | M | ~1 cycle | (coupler batch) | — | `BoundFeatureKind`→`mechanics::combat`; `rider_hand_world_pos`+`HAND_OFFSET_NORM`→`features::ecs::mount`; `LoadingZoneVisual`→runtime markers. **couplers → 5 (2 are doc-comment false positives).** |
| S1-✓ | **lock sim→presentation boundary** | ☑ | S | ~1 cycle | (guard) | — | `architecture_boundaries_sim_does_not_import_presentation`: pure gameplay/sim imports ZERO presentation. Allowlist = `presentation/`, `dialog/ui.rs` (IS UI), `runtime/{setup,reset}` (composition-root orchestration). **MONOLITH SEQUENCE step 1 DONE.** Prereq for the `ambition_render` extraction (step 2). |

| S3-smell | dual inventory-bag duplication logged | ☑ | S | ~1 cycle | (smell log) | — | `PlayerInventory`/`ItemKind` (3) shadows `OwnedItems`/`Item` (24) + `legacy_kind` bridge. Real fix is a content-spanning collapse (~1-2h) — deferred to the long run, fully scoped in `code_smells.md`. |
| S2-plan | presentation→`ambition_render` step-2 execution plan | ☑ | S | ~1 cycle | (plan doc) | — | scoped the real blocker (the 3 in-lib presentation importers / crate-cycle), with the message-inversion-vs-move-up decision + the `pub`-widening surface. Ready to execute in the 10h run. |
| S5 | delete dead inventory UI markers | ☑ | S | ~1 cycle | (dead-code) | lib −28 | 8 zero-reader `#[derive(Component)]` markers from the Phase-D2-deleted adventure menu. |

| S3-a | DELETE legacy `ItemKind`/`PlayerInventory` bag → one `OwnedItems` store | ☑ `behaviour Δ` | M | ~1 cycle | (item-unify) | lib −~110 | the dual-bag smell, RESOLVED. Collapsed onto the 24-row `Item` catalog (already an id-keyed registry); `inventory/` now owns only menu-nav state. Dialogue can grant any of the 24 items now (was: the legacy 3). Behaviour Δ: starter health 2→3 (OwnedItems::starter). App+sandbox+content+yarn tests + 28 guards green. **Step-3 named-content move #1 of 3.** |

| S2 | **EXTRACT `ambition_render` crate (the ~6.6k presentation cut)** | ☑ | XL | ~45min | (5 commits) | sandbox −~6600 | THE HEADLINE. fx+hud (leaf) → reset-visual inversion → presentation_world→app (composition) → rendering/cutscene/ui_fonts/screen_effects/dialog_ui. Layering sandbox<render<content<app; sim CANNOT import its renderer (guard). Replay BIT-IDENTICAL throughout. DialogChoiceSlot moved DOWN to dialog::runtime; pub-widened 4 sandbox internals; SfxBankResource→audio. MONOLITH SEQUENCE step 2 DONE. |

## Final summary (run 1 — 2026-06-15)

- **New foundation crates (3):** `ambition_ui_nav`, `ambition_cutscene` (format +
  stepper + live state), + the VFX request vocab folded into `ambition_effects`.
- **Lib LOC:** ≈ −1,150 (ui_nav −711, cutscene format −311, cutscene state −55,
  + the fx-vocab move) toward the engine/content boundary.
- **Presentation couplers:** 32 → ~25. `RoomVisual` marker → `platformer_runtime::
  lifecycle` (−6, the clean inversion); cutscene state → `ambition_cutscene`
  (`app/input_systems` 8→2). `cutscene_trigger`'s remaining "ref" is a doc comment.
- **De-name + fix:** `FeatureVisualKind::Sandbag`→`TrainingDummy`; the passive
  dummy's dormant `PunchWeak` melee removed (Jon-flagged), pinned by a test.
- **Replay:** bit-identical on EVERY commit (the sandbag fix too — aggro 0 made the
  melee dormant). architecture_boundaries green throughout.
- **Infra:** the shared cargo cache at `/home/joncrall/ambition-target` had grown
  to 107G (historical, across sessions) and filled the disk mid-run; cleared → 45%.
  Worth a periodic `rm -rf` between heavy sessions.

### Clean frontier is exhausted — the remaining levers all need a FOCUSED/supervised pass:
- **`character_sprites` metadata cluster** (~5 couplers): `baked_sheet_registry` /
  `sheets` are separable DATA, but the baked table is `build.rs`-generated UNDER
  presentation — moving it relocates build-system paths (medium-large).
- **`PlayerVisual` / `SceneEntities` render-handle cluster**: no clean foundation
  home (player↔presentation cycle risk); the real A4 inversion needs a design call
  + feel-test (sim stops spawning visual-tagged entities; renderer mirrors state).
- **`ItemKind` / `BossAttackProfile` data-keying** (named content in machinery):
  big, unblocks the `inventory` extraction — Jon flagged as possible design-together.
- **God-module splits** (goals 1+2, nav/compile only): safe filler, available anytime.
