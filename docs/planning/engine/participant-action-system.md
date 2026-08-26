# Participant/action system — remaining work

> **Verified against `cecd01ca` (2026-08-13).** Per-seat identity/context/device
> state, binding recipes/overrides, persisted rebinding, vendor-aware glyphs,
> inventory/debug/pause context claims, `SeatMenuFrames`, and the semantic action
> registry/module-contribution seam are implemented. Historical PA1–PA4 execution
> and decisions are archived under
> [`../../archive/planning-superseded/2026-08-13/`](../../archive/planning-superseded/2026-08-13/).

## Remaining architecture

- ▢ **Remove the seat-0 control split.** `ControlFrame`/`ControlFrameLatch` still
  carry a primary-seat special path while secondary seats use slot/seat state.
  Converge on one participant-keyed input channel without changing simulation
  semantics merely for naming symmetry.
  ⚠ **RE-READ THIS ONE BEFORE REMOVING ANYTHING (2026-08-26).** The split is
  still there and it is now DEFENDED rather than merely present: `input_systems.rs`
  carries six separate paragraphs on why the primary seat is the keyboard's, why
  `gamepad_only()` for every non-primary seat encoded the wrong thing, and why
  the primary seat *"has no equivalent hazard because GGRS overwrites"*. ⇒ the
  first question is no longer *how* to converge but whether this is still a
  SPLIT or has become a stated DESIGN — and this row's own last sentence
  (*"not merely for naming symmetry"*) is the test to apply.

- ✔ **Per-seat pause ownership — SHIPPED, verified at HEAD 2026-08-26.** This
  doc's header predates it. `PauseMenu` carries the owner
  (*"the seat currently driving the pause menu, if one owns it"*), reads
  `SeatMenuFrames`, and implements exactly the rule stated here: while OPEN it
  reads only the opening seat's frames, and while CLOSED every seat is a
  candidate with the first in slot order winning, *"which makes a simultaneous
  press deterministic rather than dependent on iteration luck"*. Guarded by
  `the_seat_that_paused_is_the_seat_that_drives_the_menu`
  (`pause_menu.rs:788`). ⚠ the global `MenuControlFrame` remains the fallback on
  purpose — a standalone demo composes the shell without the participant
  pipeline, and there the one global frame IS the only seat.

- ◐ **Dialogue through participant contexts only.** Finish the ruling that
  dialogue is per-seat by default rather than globally suspending the world.
  Experiences that intentionally stop world time should request that policy
  explicitly.

  ✔ **THE EXPLICIT-POLICY HALF SHIPPED**: `DialogueStopsTheWorld` is a resource,
  it defaults to `false`, and `GameMode::stops_the_world` consults it. Body
  simulation is not suspended either — `ambition_conversation::rules`'s own
  header says so, which is what lets contact and damage END a live conversation.

  ⛔⛔ **AND THE OTHER HALF IS BLOCKED BY ONE MODE ANSWERING TWO QUESTIONS, ONLY
  ONE OF WHICH LEARNED THE POLICY. Measured 2026-08-26:**

```text
GameMode::stops_the_world(self, policy)   Dialogue => policy.0     ← policy-aware
GameMode::allows_gameplay(self)           matches!(self, Playing)  ← unconditional
```

  ⇒ under the DEFAULT policy the world keeps running during a conversation and
  **every seat's gameplay input is refused anyway**, because the input gate never
  asks. So *"dialogue is per-seat by default"* is not merely unbuilt — it is
  currently INEXPRESSIBLE: `GameMode` is one global `State`, and the gate that
  would have to become per-seat is a bare `matches!` on it.
  ⇒ **the real remaining work is naming what a per-seat dialogue mode IS** (a
  second seat playing while the first talks needs the gate keyed by seat, not a
  policy flag on a global mode), not threading `DialogueStopsTheWorld` into a
  second function. ⚠ and the present behaviour is coherent even if it is not the
  ruling: the world runs, you cannot act, and being hit cuts the conversation
  short — do not "fix" the gate without deciding the mode.

- ▢ **Unify semantic menu activation.** Controller submit, virtual-touch submit,
  and pointer release should produce one semantic activation seam. Pointer
  press/release-with-drag-cancel is already shared; backend-specific select
  consumption remains.
  ⭐ **AND "REMAINS" NOW HAS A NUMBER, measured 2026-08-26: the shared seam has
  exactly ONE adopter.** `ambition_ui_nav::resolve_selectable_row_interaction`
  (press/release with drag cancel, `ROW_TAP_SLOP_PX`) is used only by
  `ambition_dialog` (two call sites). `ambition_game_shell` reads the raw
  `MenuControlFrame::select` flag and routes it itself at three separate places —
  `input.rs` maps it to `confirm`, `startup_acknowledge` AND `loading_continue`,
  and `pause_menu` consumes it directly. ⇒ **the seam is not missing, it is
  UNADOPTED**, which makes this a migration with a countable finish line rather
  than a design question. ⛔ mind the three shell meanings: one flag currently
  answers three different questions there, so moving it needs those to stay
  distinguishable.

- ◐ **Move directional repeat/focus/wrap behind `ambition_ui_nav`.** Preserve the
  existing navigation semantics while removing backend-specific duplication.
  ⭐ **MEASURED 2026-08-26, and the vague half is now a NAMED DIFFERENCE.** The
  crate exists and carries all three concepts: `MenuFocusState`,
  `move_next_wrapping` / `move_previous_wrapping`. `ambition_dialog` adopts
  `MenuFocusState` (runtime + systems). ⛔ `ambition_game_shell`'s pause menu does
  NOT — it keeps its own cursor and CLAMPS it
  (`menu.cursor = (menu.cursor + 1).min(rows.len() - 1)`, `pause_menu.rs:438`).
  ⇒ **two menus in one game disagree about what happens at the end of a list**:
  the dialogue picker is on the shared vocabulary and the pause menu stops dead.
  ⚠ so this is not purely a refactor — adopting the crate's own verb makes the
  pause cursor WRAP, which is a visible behaviour change and the reason it has
  not simply been done. State the wrap rule first, then move the code to it.
  ⚠ REPEAT is deliberately elsewhere and should probably stay: it lives in
  `MenuInputState::step` in `ambition_input`, driven by the user's
  `menu_repeat_initial_delay` / `menu_repeat_interval` settings — a repeat is an
  INPUT cadence, not a list-navigation rule.

- ▢ **Finish context migration.** Inventory/specialized menus beyond the current
  cue ownership, a `VEHICLE` context, and loading/retry input remain outside the
  normal participant-context path. For loading/retry, fix the schedule ownership
  seam rather than introducing a cycle.

- ✔ **Pad-specific calibration filtering with shared bindings — SHIPPED,
  verified at HEAD 2026-08-26.** Bindings remain machine-wide by decision, and
  the filters follow the pad: `filters_for_seat` resolves
  `ControlFilters::for_pad(&settings.controls, devices.gamepad_style_for(slot))`,
  and BOTH roads take it — the menu decode and the production gameplay seat loop.
  The site says why in one sentence: *"a deadzone is a fact about the stick in
  somebody's hands"*, after player two's drifty 360 pad ran on whatever suited
  player one's DualSense.
  ⚠ **one second answer survives and is worth knowing about**:
  `read_gameplay_control_frame` (no `_with_settings`) builds
  `ControlSettings::default()` and has two callers — `ambition_sim_view::facts`
  (the blink-aim preview) and the debug overlay. Harmless today because the field
  it reads is a BUTTON and buttons are not deadzoned; it becomes a real
  disagreement the day either caller reads an axis.

- ▢ **Make provider-defined semantic actions fully usable end to end.** The code
  now has `SemanticActionId`, `ActionRegistry`, `InstalledActions`, and
  `ModuleDraft::actions` (including a tested external `grapple` registration),
  but the physical input map/cue/touch path still bottoms out in the finite
  built-in platformer action enum. A provider action should be registerable,
  bindable, presentable, and consumable without editing core action vocabulary.

- ▢ **Author input schemas/assets where it improves tooling.** Do this through
  the same registry/binding model rather than adding another settings authority.

## Exit

A local participant is represented by participant/seat/channel facts rather than
by a privileged primary-player identity, and a provider can contribute a new
semantic action all the way from authoring through physical binding and UI cue to
consumption without a core-engine edit.
