# Unified Tabbed Menu — settings IR coverage diff

**Status:** recorded during Phase A (2026-06-07). Companion to
`unified_tabbed_menu.md` §5. This is the "did we miss anything" audit: every old
pause-menu settings affordance (`SettingsItem`, incl. the Developer page +
`GameplayFlashes` + page-openers) vs. whether the shared IR covers it, verified
against the *current* code — not the spec checklist alone.

## Two layers of "the IR"

Be careful: there are **two** distinct things both loosely called "the IR", and
an affordance can be in one but not the other:

1. **IR vocabulary** — `SettingsOptionId` in
   `crates/ambition_sandbox/src/menu/ir/settings.rs`. The full set of options the
   IR *knows how to label/value/apply* via `settings_menu_model` /
   `apply_settings_option`.
2. **Curated System tree** — `curated_options()` in
   `crates/ambition_sandbox/src/menu/ir/system.rs`, which is the SUBSET the cube's
   System face actually *renders* per category. It is intentionally narrower than
   the vocabulary (the omitted options have fields/effects but aren't surfaced on
   the cube yet).

The pause menu renders nearly every option; the cube renders the curated subset.
When Phase C builds the bevy_ui System tab on the curated tree, the omitted
options below will silently disappear from the grid too. **Decision recorded
per row** whether that's acceptable (defer) or the curated tree must grow.

Source files:
- pause vocabulary: `persistence/settings/model.rs` (`SettingsItem`,
  `DevToggleSnapshot`).
- IR vocabulary: `menu/ir/settings.rs` (`SettingsOptionId`).
- IR System tree: `menu/ir/system.rs` (`curated_options`, `DevToggleId`).

---

## A. Category settings (non-developer)

`✅ vocab` = `SettingsItem::shared_option_id()` maps it to a `SettingsOptionId`.
`✅ tree` = it appears in `curated_options()` for that category (cube renders it).

| Old pause row (`SettingsItem`) | IR vocab id | In curated System tree? | Decision |
|---|---|---|---|
| **Video** | | | |
| DisplayMode | `DisplayMode` | ✅ Video | render from IR |
| CameraZoom | `CameraZoom` | ✅ Video | render from IR |
| CameraAspect | `CameraAspect` | ⚠️ **not in tree** | grow Video tree OR defer (low value on cube) — record; revisit Phase C |
| CameraFraming | `CameraFraming` | ⚠️ **not in tree** | grow Video tree OR defer — record; revisit Phase C |
| Flashes | `Flashes` | ⚠️ **not in tree** | grow Video tree (accessibility — should ship) — record; revisit Phase C |
| Colorblind | `Colorblind` | ⚠️ **not in tree** | grow Video tree (accessibility) — record; revisit Phase C |
| ShowFps | `ShowFps` | ✅ Video | render from IR |
| Shaders (19 sliders, `Shader*`) | `Shader*` (all 19+) | ✅ Video (flat, after basics) | render from IR |
| **Audio** | | | |
| MasterVolume / MusicVolume / SfxVolume / Mute | `MasterVolume`/`MusicVolume`/`SfxVolume`/`Mute` | ✅ Audio | render from IR |
| **Controls** | | | |
| KeyboardPreset | `KeyboardPreset` | ✅ Controls | render from IR (3b — see §below) |
| ControllerProfile | `ControllerProfile` | ⚠️ **not in tree** | grow Controls tree OR defer — record |
| LeftStickDeadzone | `LeftStickDeadzone` | ✅ Controls | render from IR |
| RightStickDeadzone | `RightStickDeadzone` | ⚠️ **not in tree** | grow Controls tree OR defer — record |
| TriggerPress / TriggerRelease | `TriggerPress`/`TriggerRelease` | ⚠️ **not in tree** | grow Controls tree OR defer — record |
| DpadMenuNav | `DpadMenuNav` | ⚠️ **not in tree** | grow Controls tree OR defer — record |
| InvertAimY | `InvertAimY` | ⚠️ **not in tree** | grow Controls tree OR defer — record |
| DashInputMode | `DashInputMode` | ✅ Controls | render from IR |
| TouchControls | `TouchControls` | ✅ Controls | render from IR |
| MenuTapMode | `MenuTapMode` | ⚠️ **not in tree** | grow Controls tree OR defer — record |
| ResetControlFiltering | `ResetControlFiltering` | ✅ Controls | render from IR (3b — see §below) |
| **Gameplay** | | | |
| Difficulty | `Difficulty` | ⚠️ **not in tree** | grow Gameplay tree (player-facing — should ship) — record |
| Assist | `Assist` | ⚠️ **not in tree** | grow Gameplay tree (player-facing) — record |
| PlayerDamageMultiplier | `PlayerDamage` | ⚠️ **not in tree** | grow Gameplay tree — record |
| GameplayFlashes ("Flashes (gameplay)") | — (no id; 2nd surface on `video.flashes`) | n/a | **INTENTIONAL DROP** (de-dup). The single `Flashes` (Video) is canonical. See §C. |
| DebugHud | `DebugHud` | ✅ Gameplay | render from IR |
| QuestHud | `QuestHud` | ✅ Gameplay | render from IR |
| TraceAutoDump | `TraceAutoDump` | ⚠️ **not in tree** | grow Gameplay tree OR defer (dev-ish) — record |
| **Radio** | (radio rows) | ✅ Radio entry | render from IR |
| **Actions** | | | |
| ResetAllSettings | n/a (action) | ✅ System entry `ResetAllSettings` | render from IR |
| ResetSandbox | n/a (action, dev) | ✅ System entry `ResetSandbox` (dev) | render from IR |
| Quit | n/a (action) | ✅ System entry `Quit` (**added Phase A**) | render from IR |
| **Page-openers** (OpenVideo/OpenShaders/OpenAudio/OpenControls/OpenGameplay/OpenDeveloper), Back | n/a | n/a | replaced by drill/tab nav |

### Key finding: vocabulary ≠ tree

Every category option the pause menu shows IS in the IR **vocabulary** (verified:
`shared_option_id()` returns `Some` for all of them). But the IR **System tree**
the cube renders is a curated subset, so the rows marked "⚠️ not in tree" above
(CameraAspect, CameraFraming, Flashes, Colorblind, ControllerProfile,
RightStickDeadzone, TriggerPress, TriggerRelease, DpadMenuNav, InvertAimY,
MenuTapMode, Difficulty, Assist, PlayerDamage, TraceAutoDump) would be DROPPED
from the grid in Phase C if it renders the curated tree verbatim.

**Decision:** these are NOT vocabulary gaps (nothing to extend in the IR), they
are *curation* gaps. Growing `curated_options()` is a one-line-per-row change and
costs nothing (the apply/label paths already exist). Recommended for Phase C:
extend the curated tree to include the player-facing ones at minimum — the
accessibility (Flashes, Colorblind) and gameplay (Difficulty, Assist,
PlayerDamage) rows. The pure-tuning controls rows (deadzones, triggers) and
TraceAutoDump can stay deferred if Phase C wants a tight first cut. **No row is
silently lost without this recorded decision.**

### KeyboardPreset / ResetControlFiltering (stage 3b verification)

VERIFIED against `model.rs`: `SettingsItem::KeyboardPreset → Id::KeyboardPreset`
and `SettingsItem::ResetControlFiltering → Id::ResetControlFiltering` both map,
and both appear in the Controls curated tree. They ARE in the IR (vocab + tree).
The stale `TODO(stage 3b)` comment in `model.rs` claiming Shaders/KeyboardPreset
aren't yet in the IR has been **deleted** this phase, and the surrounding doc
comment corrected (Shaders + KeyboardPreset + ResetControlFiltering are no longer
in the `None` set).

---

## B. Developer page (HIGHEST-RISK PORT — explicit row-by-row diff)

Pause Developer page rows live in `SettingsItem` (lines ~142–156) and are
labelled via `DevToggleSnapshot` (aggregating `SandboxDevState` +
`DeveloperTools` + `LdtkHotReloadState`). The IR's Developer screen is built from
`DevToggleId::ALL` (15 ids), each mapped to a `DeveloperTools` field by
`dev_snapshot()` / `apply_dev_toggle()` in `lunex_kaleidoscope_app.rs`.

| Pause Developer row | pause field source | IR `DevToggleId` | Status |
|---|---|---|---|
| DebugOverlay (F1) | `dev_state.debug_enabled()` (`SandboxDevState`) | — | ❌ **MISSING from IR** |
| SlowMotion (F2) | `dev_state.slowmo` (`SandboxDevState`) | — | ❌ **MISSING from IR** |
| Inspector | `developer.inspector_visible` | `Inspector` | ✅ |
| WorldInspector | `developer.world_inspector_visible` | `WorldInspector` | ✅ |
| OverviewCamera (F5) | `developer.overview_camera` | `OverviewCamera` | ✅ |
| DebugViewMode | `developer.debug_view_mode` | `DebugViewMode` | ✅ (cycle) |
| DebugArtMode | `developer.debug_art_mode` | `DebugArtMode` | ✅ (cycle) |
| ShowHitboxes ("Custom Hitboxes") | `developer.show_feature_hitboxes` | `ShowHitboxes` → `dev.show_player_hitbox` | ⚠️ **FIELD MISMATCH** (see note) |
| FillDebugBoxes ("Debug Fills") | `developer.fill_debug_boxes` | `FillDebugBoxes` | ✅ |
| MicroGrid (8px) | `developer.show_micro_grid` | `MicroGrid` | ✅ |
| CameraFrame | `developer.show_camera_frame` | `CameraFrame` | ✅ |
| PlayerBodyProfile | `developer.player_body_profile` | `PlayerBodyProfile` | ✅ (cycle) |
| MovementProfile | `developer.movement_profile` | `MovementProfile` | ✅ (cycle) |
| LdtkAutoApply (F12) | `ldtk_reload.auto_apply` (`LdtkHotReloadState`) | — | ❌ **MISSING from IR** |

IR `DevToggleId`s with **no** pause Developer row (extra on the cube — not a
loss, just informational): `Gizmos`, `ShowHud`, `HideSprites`,
`PlaceholderSprites`.

### Developer findings (record — do NOT fix this phase)

1. **Three dev toggles MISSING from the IR Developer screen** — the highest-risk
   gap. All three are sourced from resources OTHER than `DeveloperTools`, which is
   why the IR (which only maps `DeveloperTools` fields) can't reach them:
   - **DebugOverlay (F1)** — `SandboxDevState::debug_enabled()`.
   - **SlowMotion (F2)** — `SandboxDevState::slowmo`.
   - **LdtkAutoApply (F12)** — `LdtkHotReloadState::auto_apply`.
   To port these, a later phase must add `DevToggleId::{DebugOverlay, SlowMotion,
   LdtkAutoApply}` AND extend `dev_snapshot()`/`apply_dev_toggle()` to read/write
   `SandboxDevState` + `LdtkHotReloadState` (the dev-toggle path currently takes
   only `&mut DeveloperTools`, so its signature must widen). Flagged for the
   Developer-port phase; NOT done in Phase A.

2. **ShowHitboxes field mismatch** — the pause row labels/applies
   `developer.show_feature_hitboxes` ("Custom Hitboxes"), but the IR's
   `DevToggleId::ShowHitboxes` reads/writes `developer.show_player_hitbox`
   ("Player Hitbox"). These are DIFFERENT fields. Porting the pause Developer page
   onto the IR must reconcile this (either point the IR id at
   `show_feature_hitboxes`, or add a second id for the player hitbox). Recorded;
   not reconciled this phase.

---

## C. Intentional drops (recorded, per §5)

- **GameplayFlashes** — the pause menu's "Flashes (gameplay)" row is a SECOND
  surface on the same `video.flashes` field already exposed once as `Flashes`
  (under Video). The IR models the field once. **Dropped intentionally** (de-dup);
  the single Video `Flashes` option is canonical.

---

## D. Summary — what a later phase must extend

- **IR vocabulary gaps:** none for category settings (every pause category row
  maps). The only true IR-vocabulary additions needed are the three dev toggles
  in §B.1 (`DebugOverlay`, `SlowMotion`, `LdtkAutoApply`).
- **IR System-tree (curation) gaps:** the "⚠️ not in tree" rows in §A. Grow
  `curated_options()` in Phase C for at least the player-facing ones (Flashes,
  Colorblind, Difficulty, Assist, PlayerDamage); deadzone/trigger/TraceAutoDump
  rows may stay deferred.
- **Field reconciliation:** ShowHitboxes (§B.2).
- **Intentional drop:** GameplayFlashes (§C).
- **Done this phase:** Quit to Desktop added to the IR System tree (§A actions).
