# Contextual input and island pause

Base: `7c0c36e2c6cd` (2026-11-24 source snapshot).

## Binding contract

The same interaction grammar applies to every game. Gameplay layouts can differ. The hardware map describes physical controls. The context selects the intended operation. Consumers read resolved intents, not physical keys, and must not infer pause from Back or Reset.

| Control | Gameplay | Session UI (e.g. gate) | Pause UI | Frontend |
|---|---|---|---|---|
| Enter / NumpadEnter | Pause | Confirm | Confirm | Confirm |
| Escape | Pause | Pause | Back | Back |
| Gamepad Start | Pause | Pause | Resume | Existing frontend shell-menu behavior |
| Gamepad South | Game action | Confirm | Confirm | Confirm |
| Gamepad East | Game action | Back | Back | Back |
| Backspace | No session action | Back | Back | Back |
| Android system Back | No session action | Back | Back | Back |
| Delete / Select | Development Reset | None | None | None |
| Home | Return to the party leader when available | None | None | None |
| I / R3 | Inventory shortcut | Inventory shortcut | Inventory shortcut | None |
| M / L3 | Map shortcut | Map shortcut | Map shortcut | None |
| Shift | Walk layer | None | None | None |
| Touch Menu | Pause | Pause | Resume | Existing frontend behavior |
| Touch Reset / Back position | Reset | Back | Back | Back |

Back returns to the parent page. Back at the root closes the pause menu. A pause menu interrupts, but does not destroy, a gate terminal. A button press cannot also activate a control in the new context. A held button must not become a new edge on a context or binding change. Stick tilt supplies analog walking. Do not use R3 as a universal gameplay modifier. Retain the generic ability Modifier for game-specific needs until its consumers have been audited. I/M are reserved in keyboard defaults; WASD combat conflicts move to F/C. L3/R3 shortcuts and Backspace are proposed defaults; keep them remappable. Do not make a game without a Map or Inventory page open an unrelated pause root when these keys are pressed.

## Implementation

- `ambition_input::contextual` resolves menu intents from the physical bindings and the input owner. The input sampler uses the typed Leafwing button to track held state. The runtime resolves the button state once per seat and copies the primary result into the global menu frame. The primary projection also reads the pointer wheel.
- `ambition_input::presets` and `layout` declare the shared defaults and game layouts. `PartyReturn` has its own remappable action. Controllers can bind it in the input settings. The default keyboard control is Home. The touch bindings supply the touch button actions and the reset/back label and glyph.
- `ambition_platformer2d_runtime::menu_pause` publishes the roster from session admission and simulation authority. The Ambition and shell menus publish their logical open seats after input. `ambition_platformer2d_shared_tangle::lifecycle::menu_pause` calculates nonempty unanimity. The local session mode adapter restores the interrupted gameplay mode.
- `ambition_hosted_world::menu_pause` receives guest votes on an ordered channel. The host reads the seat from the authorized connection. A change to the session, roster, island assignment, or seat owner starts a new vote epoch. The host discards old epochs and old sequences. Guests map their local input source to their assigned seat. The host and guest stop `GameplaySimulationRoot` when the host declares a session-wide pause. The message systems continue in the frame schedule.

To inspect the desktop recipes, run `cargo run -p ambition_app --bin ambition_game_bin -- --explain-bindings`. The report lists the presets, layouts, input profiles, current overrides, and touch bindings when the build includes touch support. The report shows binding intents. The active game's capabilities control which gameplay actions and pages are available.

## Remaining execution work

1. **Room execution.** `HostedWorldPlugin` enables room authority when `RewindsGuestRooms` is present. This includes the app's `--host-world` path. `room_rollback::replay_room` runs the shared schedule with other rooms hidden while it replays saved ticks. Its live counterpart runs `GameplaySimulationRoot` across all rooms. Add a live room step that owns its clock and entity and message writes. Check the global clock and progression phases in `ambition_platformer2d_runtime::schedule` and the message and write boundaries in `lifecycle::room_writes`. The gate must also preserve pending inputs, room handoff, checkpoints, and rollback state. When this step exists, apply the island decisions at its head. Test that a stopped room's clock and effects stay unchanged while a second room continues to tick. The hosted vote path can pause a session-wide authority today. Room authorities continue to run.
2. **Peer agreement.** The GGRS host prevents a local menu from pausing a remote timeline. The peer transport in `ambition_platformer2d_rollback_ggrs::peer` exchanges generation contracts. Extend that path to exchange pause levels bound to the session, authority, and membership. Agree on a confirmed execution boundary in the session bridge before stopping gameplay. Keep polling the control channel while the gameplay set is stopped. Test the boundary with two peers, delayed packets, membership changes, and resume while the gameplay clock is stopped.

A vote comes from an admitted participant's pause menu. Gate, dialogue and cutscene input capture alone leaves the vote clear. A bodyless participant stays in the roster. An unknown room assignment prevents a pause until there is sufficient authority information. The transport and UI continue to run during a pause.
