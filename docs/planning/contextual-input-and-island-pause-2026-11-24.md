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
| Delete / Select | Development Reset | None | None | None |
| I / R3 | Inventory shortcut | Inventory shortcut | Inventory shortcut | None |
| M / L3 | Map shortcut | Map shortcut | Map shortcut | None |
| Shift | Walk layer | None | None | None |
| Touch Menu | Pause | Pause | Resume | Existing frontend behavior |
| Touch Reset / Back position | Reset | Back | Back | Back |

Back returns to the parent page. Back at the root closes the pause menu. A pause menu interrupts, but does not destroy, a gate terminal. A button press cannot also activate a control in the new context. A held button must not become a new edge on a context or binding change. Stick tilt supplies analog walking. Do not use R3 as a universal gameplay modifier. Retain the generic ability Modifier for game-specific needs until its consumers have been audited. I/M are reserved in keyboard defaults; WASD combat conflicts move to F/C. L3/R3 shortcuts and Backspace are proposed defaults; keep them remappable. Do not make a game without a Map or Inventory page open an unrelated pause root when these keys are pressed.

## Implementation in this overlay

- `ambition_input::contextual`: pure contextual button policy, held physical identity tracking, static button rows from the authoritative binding map. It resolves the disputed session/UI button actions before consumers. Directional axes, raw gameplay action projection, and arbitrary chord/remap collision validation still use existing mechanisms.
- `ambition_input::presets` and `layout`: shared defaults. Layout-specific combat is retained.
- Runtime input: global and seat menu frames use the same resolver; session shortcuts suppress gameplay in their opening frame. The old physical-overlap flags are removed.
- Ambition and generic shell: use the same pause-intent interpretation. Ambition retains per-participant surfaces; the shell stores per-seat menu/cursor state and displays one foreground surface.
- `ambition_platformer2d_shared_tangle::lifecycle::menu_pause`: admitted roster, island unanimity decisions, session-wide mode adapter preserving the interrupted mode. Host roster publisher uses session admission and actual simulation-island identity.

## Completion boundary

The overlay is not compiled in the source-only authoring environment. Do not claim completed networking or independent room suspension. Session-wide local unanimity uses the existing GameMode adapter. Room authority yields per-island decisions, but there is no room execution gate connected to those decisions yet. A remote peer timeline must not be stopped from a local-only menu state; Ambition retains a conservative remote guard until the authenticated control-plane vote protocol and an agreed execution boundary exist. This is a migration boundary, not the intended final rule.

The final invariant is nonempty unanimity of admitted human participants assigned to each simulation island, regardless of transport. Remote and couch use the same policy. An absent vote is false. A participant's menu suppresses its own controls but not another player's. Menu opening is not itself a simulation state transition. Resume of any member releases the island pause. Votes must be scoped to session, membership and current authority; stale votes must not survive departure, reconnection or session replacement. Gate/dialogue/cutscene interaction alone does not request pause. A bodyless admitted participant must not disappear from the quorum. The transport/control plane must continue while simulation is stopped. RoomPace is a hitstop/clock scaling mechanism, not a safe substitute for a complete paused-island execution gate.

The accompanying Codex handoff gives concrete validation and completion steps.
