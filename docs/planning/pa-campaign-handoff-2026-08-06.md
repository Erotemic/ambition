# PA campaign handoff — 2026-08-06

Jon, verbatim: **"Yes do the architecture task PA1-PA4."** The spec is
[`engine/participant-action-system.md`](engine/participant-action-system.md)
(§9 carries dated landed/owed annotations); binding/remap detail is owned by
[`engine/character-actions.md`](engine/character-actions.md) P1/P5. This file
is the operational record: what landed, what is left, and which forks are
decisions rather than work.

**Status: PA1–PA4 are landed except for five decision-blocked items and one
unstarted half of PA3.** Everything owed is listed under "What is left".

## What landed (verify with `git log`, don't re-derive)

| commit | what |
|---|---|
| `746a95e25` | `BindingRecipe` + `rebuild_maps_from_recipes` + `sync_primary_recipe_from_settings`: every seat's map is built from a declared recipe, engine-owned rebuild. Killed the app-only `single_mut()` preset resync. |
| `cf868687e` | `SeatActiveDevices` (per-seat device fact, `machine()` projection) replaces `ActiveInputKind` + `ActiveInputMethod`; real vendor detection (`gamepad_style_of`); glyph stack moved to `ambition_input::glyphs`. |
| `8839e9fc7` | ONE gamepad label table (`glyphs::button_label(button, style)`, `PhysicalControl::label_for`); `GAMEPAD_MAP`/`action_label`/`movement_label` deleted as dead. |
| `acc50d48c` | Touch overlay is the PRIMARY seat's in both directions (pressed-state read; virtual-binding insertion). |
| `b242c7e51` | Pre-existing versus red fixed: fixture now composes `declare_versus_experience_scope` + the shell's now-public `release_departed_experience_state`. |
| `7a957d8b2` | Pre-existing monolith fixture break (`BrainSnapshot.movement_tuning`) repaired. |
| **`9d2386b2e`** | **PA1's override model.** `BindingRecipe.overrides` layered on the base; `ControlSettings.binding_overrides` persisted; `sync_primary_recipe_from_settings` folds them in. |
| **`6cfce127d`** | **PA2's claims.** An open inventory declares `INVENTORY_CONTEXT` (backend-agnostic, in `ambition_inventory_ui`); the egui inspector declares `DEBUG_CONTEXT` while it wants the keyboard. |
| **`f8c69d7ea`** | **PA3's pointer half.** `RowPress` → `PressArm<T>`; the kaleidoscope's private copy deleted; the shared bevy_ui bridge and the tab bar moved from press-activate to release-with-drag-cancel. |
| **`edf4b7e78`** | **PA4's label half.** `ControlPrompt.binding` gets the seat's vendor spelling; the sanic legend is presentation-time and follows the live bindings. |

## What each of the four turned out to be

### PA1 — the override model (landed)

`BindingRecipe` gained `overrides: Vec<BindingOverride>` (and dropped `Copy`).
`build()` applies them AFTER the base map, **in the displaced binding's place**
and **restricted to the override's own device class**. Both of those were
first-draft mistakes the tests caught:

- appending left a remapped Jump still PRINTING the gamepad button it never
  touched, because `ActionBindings::label` is the first binding. The map-level
  assertion passed throughout; only asking for the LABEL saw it;
- `clear_action` would drop keyboard AND gamepad together, so a keyboard remap
  silently unbinds the controller.

`ControlSettings.binding_overrides` is `#[serde(default)]`, so every existing
settings file still loads (the pre-existing `save_clamps_values_back_into_range_
on_load` test already covers that, since its RON has no such field).

**D1 resolved as recommended (a).** `bevy_input` is now a direct dependency of
`ambition_input` for its `serialize` feature — ⚠ the flag is named `serialize`,
not `serde`, which the recommendation got wrong. `KeyCode`/`GamepadButton`
persist as themselves. Action NAMES still cross the settings boundary as
strings, because `settings.rs` compiles WITHOUT the `input` feature where the
action enum does not exist; the resolution (`action_named`) runs through
`Reflect`, so a new action is nameable the day it is declared.

⛔ **`FromReflect` on a variant that does not exist PANICS** rather than
answering `None`. The obvious three-line name resolution turned "a settings file
from a newer build" into a crash on load; the variant is checked against
`TypeInfo` first.

### PA2 — contexts (landed except D4/D5)

⚠ **the pause menu already declared `PAUSE_CONTEXT`** before this session
(`ambition_game_shell::pause_menu::declare_pause_context`). The earlier draft of
this file said otherwise; what is genuinely underived-from is `GameMode::Paused`
itself, which is D5's question.

Two claims landed:

- **An open inventory owns the seat's input.** In `ambition_inventory_ui`, off
  `InventoryUiState.visible`, which BOTH frontends drive — so the cube and the
  grid cannot disagree and a third gets it by raising the same flag. Two things
  became right without either backend being edited: a composition with no
  `GameMode` (a demo, a capture harness) no longer routes gameplay under an open
  inventory, and on a phone `sync_touch_button_visibility_from_prompt` now hides
  the gameplay buttons that used to sit live on top of the menu.
- **A developer typing in the inspector.** `DEBUG_CONTEXT` had existed since the
  claim system landed and nothing declared it, so editing a tuning field walked
  the player off the ledge being measured. ⚠ the condition is "egui WANTS the
  keyboard", not "a panel is open" — watching values while playing is the normal
  way to use an inspector.

⚠ **what the inventory claim does NOT delete, checked rather than assumed:** the
paused path writes a MENU frame into `ControlFrame` instead of a neutral one.
Under an inventory claim that branch is skipped — and `read_menu_control_frame`
produces exactly one bit, `start_pressed`, whose only readers are the trace
recorder and its replay. The cube's own close reads `MenuControlFrame`, which is
untouched.

### PA3 — the standard UI module (pointer half landed)

`RowPress` is now `PressArm<T>`, generic over the identity — the only thing the
three surfaces genuinely disagreed about. A flat list keys on the row INDEX; the
cube keys on the ACTION its cell carries, since a face has no stable ordinal.
Neither keys on the ENTITY, which is the point: both respawn their controls
between press and release.

`release` versus `release_anywhere` is the one real behavioural split, now named
rather than implied. A list HAS evidence about where the finger came up and must
use it; a cube respawning cells under the finger does not.

The shared bevy_ui bridge — launcher, pause menu, shell cards, grid inventory —
and the tab bar moved to release-with-drag-cancel. ⛔ **that took more than an
action key, and the difference is one frame:** a control respawned during
`Update` reads `Interaction::None` until `ui_focus_system` evaluates it in the
NEXT frame's `PreUpdate`, which is indistinguishable from a control the pointer
walked off. The two are told apart by the ENTITY (changed = rebuilt, same =
left). A test that rebuilds the page mid-press pins it.

### PA4 — the cue/glyph/touch contract (landed)

`ControlPrompt.binding` is spelled in the seat's own pad vocabulary.
⚠ `devices` is a PARAMETER of `label_for_slot`, not a second call — and the
device fact is in the prompt's CHANGE GATE, not only its derive, because a
controller swap moves the spelling without moving the binding. That half was
probed with the gate removed and goes red.

The sanic legend is presentation-time now: generation writes the default
preset's answer, `refresh_sanic_control_legend` replaces it from `SeatBindings`.

⚠ **the touch bullet was already satisfied in the tree.** Visibility, labels AND
hit-testing all resolve through `touch_action_live` — the visibility pass and
`mask_unavailable` call the same function with the same two inputs, so a button
cannot be drawn-and-untouchable or touchable-and-undrawn. What `layout.rs` still
owns is fixed POSITIONS, which are a HUD design fact and not a cue fact; making
them cue-driven would be generalizing without a consumer.

## What is left

**PA1 — per-seat presets for secondary seats.** Couch seats are hardcoded
`GamepadOnly`. The recipe is the seam and is ready; the product half is **D2**.

**PA2 — two decision-blocked migrations.** `GameMode::Paused`'s own derivation
needs **D5**; the dialogue world-stop needs **D4**. Loading/retry stays
deliberately unmigrated: moving its raw read into `Consume` creates a schedule
cycle through load-presentation + shell sequence sets (spec §8).

**PA3 — the submit and directional halves, not started.**
- One semantic activation event covering controller submit and virtual-touch
  submit alongside the now-converged pointer path; today each backend consumes
  `MenuControlFrame.select` itself.
- Directional repeat/focus/wrap behind `ambition_ui_nav`. `MenuControlFrame`
  consumers keep their nav-axis semantics (locked decision 2).
- ▢ `MenuFrameCutsceneSkip` vs `MenuNavConsume` is explicitly a DECISION, not an
  edit: one `MenuFrameConsume` set is a rename plus a widening with a
  behavioural consequence at the menu-backend switch.

## Decisions

- **D1 (technical) — RESOLVED as recommended.** Overrides persist through
  `bevy_input`'s `serialize` feature. See PA1 above.
- **D2 (product): per-seat settings.** Do couch seats get their own persisted
  preset/overrides/deadzones (profiles picked at the select screen?), or do
  machine-wide settings apply to every seat? Blocks the per-seat-presets half of
  PA1 only.
- **D3 (sequencing) — proceeding as proposed.** The P5 rebind-capture UI comes
  after PA3's menu module, so the rebind rows are built ON it rather than twice.
  The model it needs is landed.
- **D4 (product, filed): dialogue world-stop default** —
  `awaiting-maintainer-decision.md`. Per-seat contexts exist; on a couch one
  player talking freezes everyone. Keep world-stop / go per-seat / policy per
  experience.
- **D5 (product): couch pause semantics.** Who may pause, and who navigates the
  pause menu. Today: any seat pauses, the world stops, the primary navigates —
  and a migration that preserves that is available without the decision.

## Traps this campaign hit (so the next one doesn't)

- ⛔ **The planning docs LAG the tree.** Five "open" candidates one morning were
  already landed, and TWO more items in the previous draft of this very file
  were already done (pause's claim, the touch hit-test contract). Grep the tree
  before believing any ▢ — including in this file.
- ⛔ **`FromReflect` on an unknown variant panics.** See PA1.
- ⛔ **A cache key must list every input the derive reads.** The prompt gained a
  device input and needed the change gate AND the presence bits widened; without
  the gate the styled-label test goes red. Same class as the rebind line above
  it, one authority further out.
- ⚠ **`rustfmt` on a file that was never formatted adds unrelated churn.** Run
  `rustfmt --check` first and format only the files whose pending diff is your
  own code; four files in this campaign were left alone for that reason.
- Feature-gated test suites lie when run bare: `cargo test -p ambition_input`
  needs `--features input` (99 vs 48); `cargo test -p ambition_touch_input`
  needs `--all-features` (44 vs 4).
- The gate is `cargo check -p ambition_app` (~25s), never per-crate; the 11s
  contracts job needs `--check` to enforce
  (`python3 scripts/check_absence_contracts.py --check`, 25/25).
  ⚠ **a new dependency edge fails it** until the sentinel's own lockfile is
  regenerated: `cd fixtures/minimal_game && cargo tree --offline` — and that
  lockfile must be committed WITH the change, which is the contract working.
- Jon, verbatim, 2026-08-06: **"Don't bother with full verification if you ran
  targeted ones."** Targeted green = done.
- `MODULES.md` is GENERATED from `//!` headers — after editing one, run
  `python3 scripts/modules_md.py --write`.
- ⚠ **the commit trailer template goes stale after a `/model` switch.** Check
  which model you are before signing; this session's template still said Fable 5
  under Opus 5.
- rust-analyzer diagnostics were noisy throughout (feature resolution); trust
  cargo, not the panel.
- This tree has CONCURRENT sessions and a dirty `tools/ambition_sprite2d_renderer`
  submodule. Re-read `git log` before assuming HEAD, and expect
  `.llm_resource_tally/` churn (never `git add -A`).

## Known red, NOT from this campaign

`app_it::rendered_identities_are_registered::every_rendered_identity_is_a_
character_the_game_can_show` fails on
`["special_patent_clerk"]`. The test parses `"character_id": "..."` out of the
renderer targets; Jon's uncommitted edit to
`tools/ambition_sprite2d_renderer/.../patent_clerk.py` writes that metadata with
SINGLE quotes, so the parser no longer sees the id and reads the waiver as
stale. It is submodule scratch work, untouched here.
