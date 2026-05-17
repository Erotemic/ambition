# Game mode and pause input gating

The sandbox now has a coarse `GameMode` state in `crates/ambition_sandbox/src/game_mode.rs`.
This is the foundation for pause, dialogue, room transitions, and future cutscenes.

`GameMode::Playing` is the only mode that consumes gameplay actions or advances gameplay simulation. In other modes, the sandbox still accepts developer hotkeys, preset switching, window mode hotkeys, HUD updates, and the pause toggle, but it does not convert Leafwing `ActionState` into an engine `InputState`.

This fixes the old pause/freeze bug where attack, blink, jump, or movement actions could still be captured while paused and then applied to the player sprite. When the mode toggles, the player entity's Leafwing `ActionState` is cleared so stale `just_pressed` edges do not leak across the pause boundary.

The intended split is:

- `GameMode` for broad app modes: playing, paused, dialogue, room transition, cutscene.
- future per-entity state machines for enemies, bosses, chests, breakables, and NPCs.
- engine gameplay systems should eventually be scheduled behind `GameMode::Playing` rather than each feature manually checking pause.

This patch keeps the current monolithic sandbox update function for minimal risk, but introduces the state boundary that future refactors can use to split systems into schedule sets.

Implementation note: `init_state::<GameMode>()` must be called after `DefaultPlugins` has been added. `DefaultPlugins` installs Bevy's `StatesPlugin`, including the `StateTransition` schedule. Calling `init_state` before that plugin exists will panic at startup.

## Presentation input must follow the same gate

Gameplay simulation and presentation both need the same pause contract. The engine-facing `ControlFrame` is only read from Leafwing `ActionState` while the mode allows gameplay, and debug combat/blink previews now follow that same rule. This prevents raw paused-mode button presses from lighting up attack or blink previews even though the simulation itself is stopped.
