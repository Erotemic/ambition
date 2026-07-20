# Participant-centered input — the persistent participant, explicit contexts, and the semantic UI path

> **State (2026-07-20): the startup/launcher vertical slice is LANDING in
> commits on main** (this doc tracks it live). Directed by GPT 5.6's
> architecture brief (2026-07-20), executed by Fable 5. Composes with — never
> replaces — the locked character-actions law
> ([ADR 0025](../../adr/0025-character-actions-input-ownership.md),
> [character-actions.md](character-actions.md)): `ControlFrame` stays the POD
> device-shaped wire format, sim-side action resolution stays deterministic,
> and `MenuControlFrame` consumers keep their nav-axis semantics.

## The universal structure

```text
physical and virtual devices
→ persistent participant            (InputParticipant — exists at boot, owns
                                     ActionState/InputMap; never on actors)
→ bindings                          (InputMap<SandboxAction>; virtual devices
                                     register leafwing input kinds)
→ logical actions                   (SandboxAction — slots, per ADR 0025)
→ active contexts                   (ContextClaim / ActiveInputContext —
                                     declared by owning surfaces, never
                                     inferred from GameMode)
→ consumer                          (shell/menus read semantic edges in
                                     InputSet::Consume; sim reads
                                     ControlFrame/SlotControls)
→ local UI command or deterministic simulation packet
```

"Participant" is Jon's canonical word: the person in front of a controller,
non-player-centric, existing before/during/after any session. Possession
changes the controlled actor; it never moves or recreates participant input
state.

## What the slice built (by commit)

### 1. The persistent participant (`e7cc2be14`)

- `ambition_input::participant`: `InputParticipant` + `ParticipantId`
  (1:1 with the sim-side `PlayerSlot`), `InputContextId` (open string ids),
  `ContextClaim` (priority + capture), `ParticipantContexts` (declare /
  retract / sync), `ActiveInputContext` + `resolve_active_input_context`.
- The host input plugin spawns the primary participant ONCE at Startup with
  `ActionState<SandboxAction>` + the preset `InputMap`;
  `attach_player_input_components` is DELETED — device state never attaches
  to `PlayerVisual`/actor entities again. Every reader re-targeted
  (populate systems, touch button visuals, dev overlay, preset re-sync).
- Context claims by their OWNING surfaces:
  - shell sequence → `shell.startup_acknowledge` (priority 300, capturing);
  - launcher → `shell.launcher` (200, capturing);
  - session lifecycle → `gameplay` (100, capturing; mirrors
    `session_world_exists` — direct-entry and shell-gated hosts alike).
  No claim → disabled: every routed output stays neutral.
- `InputSet` grew from one `Populate` stage to the chained pipeline
  `Collect → ResolveActions → ResolveContext → Route → PublishCues →
  Consume` (`Populate` renamed `Route`). Device adapters complete before
  routing; routed semantics complete before consumers — same frame, no
  "the edge may arrive one frame later".
- `ControlFrame` production gates on the gameplay context: the launcher
  structurally CAPTURES gameplay actions. In-session pause/dialogue/
  cutscene suppression is unchanged (those are states of the session's own
  context, not migrated surfaces).

### 2. Shell semantic consumption + cues (this commit)

- `shell_action_edges` reads ONLY `MenuControlFrame` (always live now — the
  participant exists at the title screen). Raw keyboard/gamepad reads,
  `ShellAnalogLatch`, and the write-only `ShellInputFocus` are DELETED.
  Shell/pause/load consumers run in `InputSet::Consume`; launcher and
  sequence command processing is pinned after `Consume` (same-frame
  press → cursor move / launch / card advance).
- Vanity/startup cards are tap-anywhere: the card root is a pressable
  surface flowing through the SAME `MenuActionActivated` bridge and the
  SAME `ShellSequenceCommand` as keyboard/controller confirm — the
  architecture fixes it, not a special case.
- Cues: `ambition_input::cues::{UiCue, ActiveUiCues}` — an open,
  context-keyed vocabulary. The shell presentation publishes its surfaces'
  verbs ("Continue"; "Play"/exit label per focused row); the inventory
  provider publishes "Equip"/"Use" under its own minted context id.
  `MenuConfirmPrompt` is DELETED.
- `ControlPrompt` stays the ONE presenter-facing read-model, with one
  writer per frame decided by the resolved context:
  `publish_frontend_context_prompt` (Update, PublishCues→Consume window)
  writes it while a frontend context owns input; `rebuild_control_prompt`
  (sim tail) yields on exactly those frames and folds the top cue for
  in-session menu contexts. The touch overlay needed zero changes to show
  a labeled confirm button at the launcher.

### 3. Touch as a virtual device (`fc37545b2`)

- `MobileTouchState` is a leafwing input SOURCE: custom
  `UpdatableInput`/`Buttonlike`/`DualAxislike` kinds
  (`ambition_touch_input::virtual_device`) over the touch stick and
  buttons, registered with `register_input_kind` and BOUND in the
  participant's `InputMap` (stick → `Move` + `MenuStick` + the
  `MoveLeft/Right/Up/Down` direction-threshold buttons at the gamepad's
  0.5 threshold; Jump/Interact → gameplay verb + `MenuSelect`; Reset →
  `Reset` + `MenuBack`; Start → `Start`; FlyToggle → `Utility`; Shield →
  `QuickAction`). The Jump-button-as-menu-confirm behavior is a DECLARED
  binding, not a secret branch; a preset swap re-binds automatically.
- `fold_to_control_frame` / `fold_to_menu_control_frame` and their
  `GameMode` gates (`allows_gameplay`, `menu_move_active`) are DELETED —
  context routing happens downstream in the one populate path, which also
  unifies deadzones and cutscene suppression across devices. Touch state
  collection moved to PreUpdate (before leafwing's unify): a touch press
  this frame is an ActionState press this frame. Drag-scroll stays as a
  small pointer-gesture lane (`scroll_y`), like the mouse wheel; the knob
  input-display override keys on `ControlPrompt`'s context, not GameMode.
- Deliberate feel change (flagged for Jon): touch-vs-keyboard axis
  exclusivity becomes leafwing's standard multi-device aggregation
  (sum-then-clamp) — touch now coexists with the keyboard exactly the way
  the gamepad already does. The old "touch wins" rule existed only because
  touch bypassed the bindings layer. Veto point if the feel regresses:
  the `TouchVirtualStick` binding.

### 4. Assembled behavioral tests (`app_it::participant_input`)

The REAL shell-host composition + the REAL host input stack + the touch
virtual device, headless, no gameplay actor at boot: startup card owns the
context and cues "Continue"; tap-anywhere on the card advances it through
the same semantic command as confirm; a confirm held across the
card→launcher transition does not launch; keyboard/gamepad/touch-stick all
move the launcher selection; the launcher captures gameplay actions and
cues "Play"; the touch confirm button activates the selected route.
Source-ownership: session activation/teardown never recreates the
participant, no `ActionState` on actor/visual entities, the same
participant drives the replacement actor. Frame rawness: keyboard right,
pad right, and touch-stick right reach the gameplay `ControlFrame` as the
IDENTICAL raw screen axis; a confirmation held into gameplay never
surfaces as a jump press edge. (Pointer row activation and the cue fold
are covered at crate level: `ambition_game_shell::semantic_input_tests`,
`ambition_sim_view::control_prompt`, `ambition_touch_input::tests`.)

## Reference-frame invariants (Jon, binding)

The participant/action layers own device-independent intent, NEVER spatial
interpretation. Movement/aim axes stay raw `ScreenAxes` from the device all
the way to `AccelerationFrame::resolve_control` at its two call sites
(`brain/player.rs`, `affordances/intent.rs`); `InputFrameMode`,
`ControlFrameModes`, and per-body `ResolvedMotionFrame` semantics are
untouched. "Resolved action" means bindings+context resolved — never
axes transformed. Any change there is a regression unless authorized.

## Author-facing endpoint (direction, not built)

`PlatformerInputPlugin::standard()`-shaped setup; games contribute action
schemas without editing `SandboxAction` (the enum is retained internally for
this slice, explicitly NOT the permanent extension mechanism); acknowledge
surfaces / standard menus as spawnable components. Do not build ahead of
consumers.

## Deliberately not done (this slice)

Rebinding UI (P5 stands), inventory/specialized menu migration beyond cues,
`ControlFrame` redesign, multiple local participants (ParticipantId exists;
frames are still single-slot), deterministic cutscene input (cutscene
design-first track), dialogue/pause/vehicle contexts (the stack is open for
them), editor-authored input assets, loading-context migration (its `retry`
keeps a local raw read until that surface migrates).
