# Developer hotkeys

Developer keyboard policy has one owner:

```text
crates/ambition_platformer_primitives/src/developer_hotkeys.rs
```

That module maps exact physical chords to semantic `DeveloperAction` messages.
Simulation, presentation, shell, and tooling systems consume those actions and
must not read function keys directly. This keeps temporary debugging priorities
editable in one file and prevents two subsystems from silently claiming the
same key.

## Current deck

| Chord | Action |
|---|---|
| `F1` | toggle the global debug overlay |
| `F2` | toggle developer slow motion |
| `F3` | toggle the resource inspector |
| `F4` | toggle the world inspector |
| `F5` | toggle the overview camera |
| `F6` | toggle the FPS overlay |
| `F7` | toggle the portal gun |
| `F8` | request one gameplay trace dump |
| `Shift+F8` | request one portal view-cone dump |
| `F9` | request one bounded GGRS rollback proof |
| `F10` | quit the active session to its home route |
| `F11` | validate and apply pending LDtk content |
| `F12` | toggle LDtk auto-apply |

Borderless fullscreen and portal mapping convention have no developer
shortcuts. Display mode remains a user setting; portal convention remains a
code/configuration choice rather than a live gameplay toggle.

## Rules

- Add or change physical chords only in `DeveloperHotkeyBindings::default`.
- Consumers match `DeveloperAction`, never `KeyCode::F*`.
- Chords are exact: extra Shift/Ctrl/Alt modifiers suppress an unmodified
  binding, so `F8` and `Shift+F8` cannot both fire.
- The plugin validates that every action and chord appears at most once.
- Irreversible effects triggered by an action, such as writing trace files,
  execute outside the rollback simulation schedule.

The bindings are a Bevy resource, so a future developer-settings loader can
replace the default deck before `DeveloperHotkeyPlugin` builds without changing
consumer systems.
