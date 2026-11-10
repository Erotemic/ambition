# Smash defect review — 2026-10-31

**Source read:** `5ec96e4e3152` (2026-10-31). **State:** SD-01 through SD-05
fixed 2026-11-10, each with a test that a poison of its fix reddens (see
"Fixed" under each). SD-06 and SD-07 were measured and fixed the same day,
with before and after counts under each. This review records defects and possible performance costs.
It does not track Smash feature parity or game feel.

**Tracking:** [SMASH-DEFECTS in the live queue](../queue.md#smash-defects--select-screen-respawn-and-result-lifecycle)
owns execution and status. This document owns the source findings, reproduction
steps and acceptance details. The maintainer selected queue triage on
2026-10-31. Recheck current source before each fix. The existing
[`S0.*` combat work order](super-smash-siblings.md#start-here) remains separate.

## Findings

SD-01 through SD-05 describe correctness candidates. SD-06 and SD-07 require
cost measurements. Their evidence and acceptance criteria follow below.

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

**Fixed 2026-11-10.** `place_the_screen` hides every portrait the layout puts
on another page (`Display::None`, so its children go too). Witness:
`page_tests::only_the_current_pages_portraits_are_drawn` (320x240, pages 0,
1 and 0 again).

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

**Fixed 2026-11-10.** `place_respawning_fighters` passes the body's own
`MotionModel::air_jumps()`. Witness: `tests::a_respawn_gives_each_fighter_its_own_air_jumps`
(fighters authored with 0, 1 and 3 air jumps; the engine default made the
0 come back as 1).

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

**Fixed 2026-11-10.** The countdown holds while `GameMode` is `Paused` (the
shell's pause menu pauses the mode). Witness:
`tests::a_paused_result_card_waits_for_the_game` (paused for four delays the
card stays; after resuming, select follows the whole delay and not half of it).

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

**Fixed 2026-11-10.** `drive_the_cursor` remembers which select-route arrival
(`ShellActivationId`) its last mouse position belongs to; a new arrival
counts the pointer's place as a move once. Witness:
`pause_arbitration_tests::a_still_mouse_places_the_cursor_on_each_arrival`
(control: within one arrival a still mouse does not move the cursor).

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

**Fixed 2026-11-10.** Two page arrows (`Anchored::Page`) are drawn at
`SelectLayout::page_button`, the rectangles of the `PagePrev`/`PageNext`
targets, only while there is more than one page; the next arrow shows
"page/pages". Witness: `page_tests::the_page_arrows_are_drawn_where_their_targets_are`.

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

**Fixed 2026-11-10 (measured).** The cause was as read: callers passed
`&mut` of a `Mut<T>`, and that dereference marks the component changed before
the helper compares. The helpers now take the `Mut` and write only a new value
(`set_if_neq`, or a compare through `Deref` for the four `Node` fields).
Witness: `an_idle_select_screen_writes_nothing::an_idle_select_screen_marks_nothing_changed`
in `ambition_demo_smash_app`, on `build_demo_app` with one pad, counting
changed components on the screen's pieces in `Last` over 120 idle frames.

| | `Node` | `BorderColor` | `Visibility` | whole-app frame (headless debug) |
|---|---|---|---|---|
| before, each idle frame | 22 | 9 | 16 | 1272, 1059, 1140 us |
| after, each idle frame | 0 | 0 | 0 | 1038, 1023 us |

Control: a moved cursor marks its node on the next frame. The frame times do
not separate: the headless foundation has no `UiPlugin`, so the layout pass
the stray changes triggered does not run there. No frame-cost win is claimed;
the saving is in Bevy's UI layout on a windowed host, unmeasured.

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

**Fixed 2026-11-10 (measured).** Each card carries `ShownPortrait`, the look
(fighter or skin id, or the random square) its picture was resolved for. The
card resolves again only when that look changes, or when `SpriteContent`, the
catalog or the entity sheets change (an existing reload path).
Measured with a temporary counter in `portrait_art` (not committed: the image
ledger is process-wide, so a counting gate would read other tests' demands),
two runs each, over 120 idle frames on `build_demo_app`:

| occupied cards | resolutions before | frame before | resolutions after | frame after |
|---|---|---|---|---|
| 1 | 120 | 9689, 9160 us | 0 | 1197, 1041 us |
| 4 | 360 (one pick has no portrait) | 18014, 17397 us | 0 | 1050, 1084 us |

The source of the frame cost: on this VM two of the picked portraits are not
published (`sprites/george_booul_portraits.png` and
`sprites/player_robot_v2_portraits.png` are `NotFound`), and a `load` of a
path whose load failed starts the load again, about 8 ms a frame for each such
card. The card whose portrait loaded cost nothing measurable. So the size of
the cost depends on the art present; the repeat demand happens everywhere.
Witness for what the cache must not break:
`a_card_shows_the_look_it_holds_after_the_look_changes` (another fighter, a
skin, back again, and an empty card). A poison that keyed the cache on "a pick
exists" instead of the look reddened it.

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
