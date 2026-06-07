# Unified Tabbed Menu — settings IR coverage diff

**Status:** recorded during Phase A (2026-06-07); **gaps closed in Phase C1
(2026-06-07)** — see the ✅ resolutions inline + the §D update. Companion to
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
| CameraAspect | `CameraAspect` | ✅ Video (**Phase C1**) | render from IR |
| CameraFraming | `CameraFraming` | ✅ Video (**Phase C1**) | render from IR |
| Flashes | `Flashes` | ✅ Video (**Phase C1**) | render from IR |
| Colorblind | `Colorblind` | ✅ Video (**Phase C1**) | render from IR |
| ShowFps | `ShowFps` | ✅ Video | render from IR |
| Shaders (19 sliders, `Shader*`) | `Shader*` (all 19+) | ✅ Video (flat, after basics) | render from IR |
| **Audio** | | | |
| MasterVolume / MusicVolume / SfxVolume / Mute | `MasterVolume`/`MusicVolume`/`SfxVolume`/`Mute` | ✅ Audio | render from IR |
| **Controls** | | | |
| KeyboardPreset | `KeyboardPreset` | ✅ Controls | render from IR (3b — see §below) |
| ControllerProfile | `ControllerProfile` | ✅ Controls (**Phase C1**) | render from IR |
| LeftStickDeadzone | `LeftStickDeadzone` | ✅ Controls | render from IR |
| RightStickDeadzone | `RightStickDeadzone` | ✅ Controls (**Phase C1**) | render from IR |
| TriggerPress / TriggerRelease | `TriggerPress`/`TriggerRelease` | ✅ Controls (**Phase C1**) | render from IR |
| DpadMenuNav | `DpadMenuNav` | ✅ Controls (**Phase C1**) | render from IR |
| InvertAimY | `InvertAimY` | ✅ Controls (**Phase C1**) | render from IR |
| DashInputMode | `DashInputMode` | ✅ Controls | render from IR |
| TouchControls | `TouchControls` | ✅ Controls | render from IR |
| MenuTapMode | `MenuTapMode` | ✅ Controls (**Phase C1**) | render from IR |
| ResetControlFiltering | `ResetControlFiltering` | ✅ Controls | render from IR (3b — see §below) |
| **Gameplay** | | | |
| Difficulty | `Difficulty` | ✅ Gameplay (**Phase C1**) | render from IR |
| Assist | `Assist` | ✅ Gameplay (**Phase C1**) | render from IR |
| PlayerDamageMultiplier | `PlayerDamage` | ✅ Gameplay (**Phase C1**) | render from IR |
| GameplayFlashes ("Flashes (gameplay)") | — (no id; 2nd surface on `video.flashes`) | n/a | **INTENTIONAL DROP** (de-dup). The single `Flashes` (Video) is canonical. See §C. |
| DebugHud | `DebugHud` | ✅ Gameplay | render from IR |
| QuestHud | `QuestHud` | ✅ Gameplay | render from IR |
| TraceAutoDump | `TraceAutoDump` | ✅ Gameplay (**Phase C1**) | render from IR |
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

**Decision:** these were NOT vocabulary gaps (nothing to extend in the IR), they
were *curation* gaps. **RESOLVED in Phase C1:** `curated_options()` in
`menu/ir/system.rs` was grown to the FULL player-facing set, in pause-menu page
order — Video gained CameraAspect/CameraFraming/Flashes/Colorblind (then shaders
still ride after); Controls gained ControllerProfile/RightStickDeadzone/
TriggerPress/TriggerRelease/DpadMenuNav/InvertAimY/MenuTapMode; Gameplay gained
Difficulty/Assist/PlayerDamage/TraceAutoDump. The apply/label paths already
existed (every id is in the IR vocabulary), so this was a pure curation change.
A new test (`system_screens_surface_every_player_facing_setting`) asserts the
full expected id set per screen. **No row is lost when the pause menu is deleted.**

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
| DebugOverlay (F1) | `dev_state.debug_enabled()` (`SandboxDevState`) | `DebugOverlay` (**Phase C1**) | ✅ |
| SlowMotion (F2) | `dev_state.slowmo` (`SandboxDevState`) | `SlowMotion` (**Phase C1**) | ✅ |
| Inspector | `developer.inspector_visible` | `Inspector` | ✅ |
| WorldInspector | `developer.world_inspector_visible` | `WorldInspector` | ✅ |
| OverviewCamera (F5) | `developer.overview_camera` | `OverviewCamera` | ✅ |
| DebugViewMode | `developer.debug_view_mode` | `DebugViewMode` | ✅ (cycle) |
| DebugArtMode | `developer.debug_art_mode` | `DebugArtMode` | ✅ (cycle) |
| ShowHitboxes ("Custom Hitboxes") | `developer.show_feature_hitboxes` (+ `show_player_hitbox`) | `ShowHitboxes` → BOTH fields + `mark_debug_view_custom()` (**Phase C1**) | ✅ |
| FillDebugBoxes ("Debug Fills") | `developer.fill_debug_boxes` | `FillDebugBoxes` | ✅ |
| MicroGrid (8px) | `developer.show_micro_grid` | `MicroGrid` | ✅ |
| CameraFrame | `developer.show_camera_frame` | `CameraFrame` | ✅ |
| PlayerBodyProfile | `developer.player_body_profile` | `PlayerBodyProfile` | ✅ (cycle) |
| MovementProfile | `developer.movement_profile` | `MovementProfile` | ✅ (cycle) |
| LdtkAutoApply (F12) | `ldtk_reload.auto_apply` (`LdtkHotReloadState`) | `LdtkAutoApply` (**Phase C1**) | ✅ |

IR `DevToggleId`s with **no** pause Developer row (extra on the cube — not a
loss, just informational): `Gizmos`, `ShowHud`, `HideSprites`,
`PlaceholderSprites`.

### Developer findings — ✅ RESOLVED in Phase C1

1. **Three dev toggles** (DebugOverlay/SlowMotion/LdtkAutoApply) — these are
   sourced from resources OTHER than `DeveloperTools`, so the old IR (which mapped
   only `DeveloperTools` fields) couldn't reach them. **RESOLVED:** added
   `DevToggleId::{DebugOverlay, SlowMotion, LdtkAutoApply}` (in `menu/ir/system.rs`,
   `ALL` is now 18) and WIDENED the dev-toggle path in `lunex_kaleidoscope_app.rs`:
   `dev_snapshot`/`apply_dev_toggle` now take a `DevToggleRead`/`DevToggleWrite`
   bundle of `&[mut] DeveloperTools` + `&[mut] SandboxDevState` +
   `&[mut] LdtkHotReloadState`. `SystemMenuParams` + `SystemMenuSnapshotParams`
   gained the two extra resources. The apply arms mirror the pause menu exactly:
   `DebugOverlay → dev_state.debug`, `SlowMotion → dev_state.slowmo`,
   `LdtkAutoApply → ldtk_reload.auto_apply` (+ the same `last_status` line).
   A World-style test (`extra_dev_toggles_flip_their_non_developer_resources`)
   asserts each flips the right resource.

2. **ShowHitboxes field mismatch** — the pause row applied
   `developer.show_feature_hitboxes` (+ `show_player_hitbox`) and called
   `mark_debug_view_custom()`, but the IR id flipped only `show_player_hitbox`.
   **RESOLVED:** `DevToggleId::ShowHitboxes` now does EXACTLY what the pause row
   does — `mark_debug_view_custom()` then flip BOTH `show_feature_hitboxes` and
   `show_player_hitbox` together; the snapshot reads `show_feature_hitboxes`
   (the pause source). Relabelled "Player Hitbox" → "Custom Hitboxes" to match.
   Asserted by `show_hitboxes_toggles_feature_and_player_fields_like_pause`.

---

## C. Intentional drops (recorded, per §5)

- **GameplayFlashes** — the pause menu's "Flashes (gameplay)" row is a SECOND
  surface on the same `video.flashes` field already exposed once as `Flashes`
  (under Video). The IR models the field once. **Dropped intentionally** (de-dup);
  the single Video `Flashes` option is canonical.

---

## D. Summary — what a later phase must extend

- **IR vocabulary gaps:** ✅ CLOSED (Phase C1). The three dev toggles
  (`DebugOverlay`, `SlowMotion`, `LdtkAutoApply`) are now in `DevToggleId` and the
  dev-toggle path was widened to their source resources.
- **IR System-tree (curation) gaps:** ✅ CLOSED (Phase C1). `curated_options()`
  grew to the FULL player-facing set per category (no deferral) — see the §A
  ✅ rows and the Key-finding update.
- **Field reconciliation:** ✅ CLOSED (Phase C1). ShowHitboxes now flips both
  hitbox fields + marks the debug view custom, at pause-menu parity (§B.2).
- **Intentional drop:** GameplayFlashes (§C) — still an intentional de-dup.
- **Done in Phase A:** Quit to Desktop added to the IR System tree (§A actions).

**Phase C1 status:** every recorded gap is resolved. The System menu surfaces the
SAME complete settings the old pause menu does (verified by
`system_screens_surface_every_player_facing_setting`,
`developer_screen_surfaces_the_three_extra_resource_toggles`,
`extra_dev_toggles_flip_their_non_developer_resources`,
`show_hitboxes_toggles_feature_and_player_fields_like_pause`, plus the existing
`shared_ir_parity` + cube reachability tests). Nothing is lost when the pause
menu is later deleted.
