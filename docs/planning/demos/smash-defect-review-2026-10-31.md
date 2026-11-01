# Smash defect review — 2026-10-31

**Source read:** `5ec96e4e3152` (2026-10-31). **State:** open findings; not
runtime-confirmed. This review records defects and possible performance costs.
It does not track Smash feature parity or game feel.

**Tracking:** Standalone review and TODO list. All task statuses are owned by
this document; do not copy these tasks into or edit the shared planning queue.
Recheck the current source before each fix. This defect review does not change
the existing [`S0.*` combat work order](super-smash-siblings.md#start-here).

## TODO (standalone)

- [ ] **SD-01 (P1):** Hide off-page portraits on character-select page changes.
- [ ] **SD-02 (P2):** Preserve fighter-specific air-jump budgets after respawn.
- [ ] **SD-03 (P2):** Stop the match-results return countdown while paused.
- [ ] **SD-04 (P2):** Resynchronize mouse position when re-entering character select.
- [ ] **SD-05 (P2):** Display page navigation controls that match their hit targets.
- [ ] **SD-06 (P2, investigate):** Measure and avoid unnecessary UI change ticks.
- [ ] **SD-07 (P2, investigate):** Measure and avoid repeated idle portrait requests.

Each item is expanded below with evidence, reproduction steps, proposed fix,
and acceptance criteria. Mark checkboxes here as the work is completed.

## Review rules

- Use the real select screen, match state, and engine services. Do not add a
  second code path for a test.
- Write a focused test that fails on the defect before a fix. A code read is
  evidence for an investigation, not proof that a player has seen the defect.
- Measure a performance cost before a broad change. Keep good cache behavior
  and do not rebuild art or Rust code just to run these tests.
- Preserve input from keyboard, mouse, touch, and pads. Preserve match rollback
  and the rules of each fighter.

## SD-01 — Portraits from the prior page stay on screen (P1)

**Owner:** Smash select-screen UI.

**Source:** `game/ambition_demo_smash/src/select_screen.rs:1475-1498` and
`game/ambition_demo_smash/src/select_screen/layout.rs:189-213`.

`place_the_screen()` sets a portrait rectangle only when the portrait is on the
current page. When the page changes, the old portrait node keeps its prior
rectangle. The layout excludes the old cell from hit tests, but the render path
does not hide it. This can put old pictures over the next page.

**Reproduce:** Use a roster with more than one portrait page at a small
viewport. Show page 0, move to page 1, and inspect the visible portrait cells.
Check that the hit targets and the rendered cells match.

**Fix direction:** Hide cells outside `SelectLayout::page_range()`. Show them
again when their page becomes active. Use the same page model for input and
presentation.

**Acceptance:** No off-page portrait or child is visible. Page changes restore
the right cells. Input and rendering select the same cells.

## SD-02 — Respawn can change the fighter's air-jump count (P2)

**Owner:** Smash stock and respawn placement, with the shared body reset API.

**Source:** `game/ambition_demo_smash/src/lib.rs:854-897` and
`crates/ambition_platformer2d_core/src/movement/model.rs:915-930`.

`place_respawning_fighters()` passes `DEFAULT_TUNING.air_jumps` to
`reset_body_clusters()`. `MotionModel::air_jumps()` already reports the body's
authored jump budget. A fighter can therefore receive the wrong jump count
after a stock loss.

**Reproduce:** Spawn fighters with air-jump limits of 0, 1, and 2. Spend their
jumps, lose a stock, and inspect the reset budget.

**Fix direction:** Pass the active body's jump budget to the reset operation.
Do not replace the fighter's movement policy with a Smash-only default.

**Acceptance:** A fighter gets its own jump budget on initial spawn and each
respawn. The count does not depend on the stock number or seat.

## SD-03 — The result timeout may advance during pause (P2)

**Owner:** Smash match completion and shell pause state.

**Source:** `game/ambition_demo_smash/src/lib.rs:1143-1223`.

The result countdown subtracts `Time::delta_secs()` in `Update`. The code does
not check the pause state before it subtracts time. Confirm how the shell pause
state and the route interact in a running match.

**Reproduce:** End a match, open pause while the result is still shown, and
advance real time beyond `return_to_select_seconds`. Resume.

**Fix direction:** Hold the countdown while the shell is paused. Keep its
existing session and route reset behavior.

**Acceptance:** Pause time does not consume the result delay. The game returns
to select only after the unpaused delay. An abandoned match still returns by
its current immediate path.

## SD-04 — A stationary mouse can target the wrong cell on re-entry (P2)

**Owner:** Smash select input and route activation.

**Source:** `game/ambition_demo_smash/src/lib.rs:1663-1688` and
`game/ambition_demo_smash/src/select_screen.rs:927-931,1082-1102`.

A new select route resets `SelectCursors`, but `SelectDriverLocal.last_mouse`
keeps its prior position. The input driver moves a cursor only when the physical
mouse position changes. A click after re-entry can therefore use the reset
cursor's position instead of the pointer's position.

**Reproduce:** Point at a fighter, start and end a match, then click that
fighter without moving the mouse. Compare the click target with the pointer.

**Fix direction:** Sync mouse position at each select-route activation. Keep
the movement threshold during normal play so a stationary mouse does not
compete with pad or keyboard navigation.

**Acceptance:** The first click after re-entry selects the cell under the
pointer. Keyboard and pad navigation still work while the mouse is still.

## SD-05 — Page input targets have no matching visible controls (P2)

**Owner:** Smash select-screen UI.

**Source:** `game/ambition_demo_smash/src/select_screen/layout.rs:262-274,365-395`
and `game/ambition_demo_smash/src/select_screen.rs:483-900,1368-1373`.

`SelectLayout::targets()` adds `PagePrev` and `PageNext` when the roster needs
multiple pages. The input handler accepts those targets. The UI spawn code
has no matching page buttons. Touch and mouse users must find the hit regions
without a visible control.

**Reproduce:** Open a multi-page roster on a phone-size viewport. Use only
mouse or touch to change pages.

**Fix direction:** Draw page controls at the positions the layout defines.
Show the current page and page count. Do not invent a second hit layout.

**Acceptance:** A visible page control maps to each active hit target. Controls
are absent or disabled when only one page exists. Mouse, touch, and pad
navigation all reach every page.

## SD-06 — Idle UI projection may mark components changed (P2; measure)

**Owner:** Smash select-screen presentation.

**Source:** `game/ambition_demo_smash/src/select_screen.rs:1475-1527,1551-1680,1784-1809`.

The projection systems run on each update. Their helpers receive `&mut Node`,
`&mut BorderColor`, and `&mut Visibility`. Taking mutable access through Bevy's
`Mut<T>` can set a change tick before the helper compares values. That may
cause extra UI layout or downstream work on an idle screen.

**Measure:** Count changed `Node`, `BorderColor`, and `Visibility` components
and measure select-screen frame time when no input or asset changes occur.
Include a control case with actual layout changes.

**Fix direction:** Compare before mutable dereference, or use Bevy's
`set_if_neq()` where it fits. Keep the existing visual output.

**Acceptance:** Unchanged frames do not report these components as changed
solely because the projection runs. Report before/after change counts and frame
cost. Do not claim a performance win without measurements.

## SD-07 — Slot portraits request the same images on idle frames (P2; measure)

**Owner:** Smash select-screen presentation and asset handles.

**Source:** `game/ambition_demo_smash/src/select_screen.rs:1551-1667` and
`crates/ambition_sprite_sheet/src/game_assets/mod.rs:34-55`.

`sync_select_cards()` calls `art.portrait()` for a selected slot on each update.
This can call `load_sheet_image()` again. The loader records demand each time,
even if Bevy returns a cached image handle. This is redundant request work,
not evidence of repeated image decoding.

**Measure:** Count portrait load calls and asset-demand records over a fixed
idle interval, with 1 and 4 occupied cards. Confirm the source of any frame
cost before changing how images are loaded.

**Fix direction:** Keep a stable handle and image rectangle per selected
character and skin. Refresh when a choice changes or when an existing reload path changes the
selected asset generation. Do not add a hot-reload requirement.

**Acceptance:** Idle cards do not issue repeat image demands. Character and skin changes
update the picture. A new published asset is visible after restart (or an
existing supported reload). Report measured cost before and after the change.

## Focused checks

Run the smallest tests that cover the affected production paths. Add headless
select-screen and respawn tests at the owning scope. The existing Smash test
lane is:

```sh
cargo test -p ambition_demo_smash_app
```

Use a small-viewport UI test for SD-01 and SD-05. Use a real stock reset for
SD-02. Exercise route activation and pause state for SD-03 and SD-04. Collect
idle and active-frame measurements for SD-06 and SD-07. Do not make a full
workspace build, asset regeneration, or publication audit a routine condition
for working on these defects.
