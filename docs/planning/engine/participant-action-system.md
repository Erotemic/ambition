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

- ▢ **Dialogue through participant contexts only.** Finish the ruling that
  dialogue is per-seat by default rather than globally suspending the world.
  Experiences that intentionally stop world time should request that policy
  explicitly.

- ▢ **Unify semantic menu activation.** Controller submit, virtual-touch submit,
  and pointer release should produce one semantic activation seam. Pointer
  press/release-with-drag-cancel is already shared; backend-specific select
  consumption remains.

- ▢ **Move directional repeat/focus/wrap behind `ambition_ui_nav`.** Preserve the
  existing navigation semantics while removing backend-specific duplication.

- ▢ **Finish context migration.** Inventory/specialized menus beyond the current
  cue ownership, a `VEHICLE` context, and loading/retry input remain outside the
  normal participant-context path. For loading/retry, fix the schedule ownership
  seam rather than introducing a cycle.

- ▢ **Pad-specific calibration filtering with shared bindings.** Bindings remain
  machine-wide by decision; deadzones/trigger thresholds should follow the actual
  controller/pad.

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
