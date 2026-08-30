# Developer hotkeys

Developer keyboard policy has one owner:

```text
crates/ambition_platformer2d_shared_tangle/src/developer_hotkeys.rs
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
| `Shift+F6` | toggle the gamepad-axis probe (and reset its peaks) |
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


## The gamepad-axis probe (`Shift+F6`)

What each connected pad's LEFT STICK actually reports, per pad, with a
**peak-hold** — because nobody can hold a stick at its true maximum and read a
screen at the same time. Push the stick to each corner, then look. Toggling
resets the peaks, so a second run is a fresh measurement, and toggling OFF also
writes the whole readout to the log (`ambition::gamepad_probe`) for a host where
reading an overlay with both thumbs occupied is harder than it sounds.

**Why it exists.** A Smash attack needs a left-stick FLICK, and
`AttackGestureTuning::flick_threshold` is `0.8` applied AFTER the inner deadzone:

```text
post = (raw - deadzone) / (1 - deadzone)
```

A Switch Pro is detected as `GamepadStyle::Switch`, which takes the `Generic`
profile's baseline `0.18`. So a flick needs **0.836 raw** while an ordinary
directional tilt needs about **0.59 raw**. A pad that tops out near 0.80 on one
host runs, tilts and drives menus perfectly while Smash attacks *cannot exist* —
and the same pad on a host reaching 0.95+ works fine. The probe prints a verdict
line saying which of those it is looking at.

⭐ It reads Bevy's `Gamepad` **directly**, not the action layer: the question is
about the DEVICE, and routing it through bindings, seats and action state would
put four more suspects between the stick and the number.

⚠ It DIAGNOSES; it does not calibrate. If a peak comes back under 0.836 the
repair is an OUTER saturation stage at the shared input seam — an `outer`
alongside the inner deadzone, so `0.8` means the same gesture on every pad — and
not a weaker Smash threshold to suit one host. The right outer value is whatever
this measures, which is why the measurement comes first.

You can corroborate outside Ambition with `jstest` / `evdev-joystick`: a
calibrated stick should approach the full ±32767 range.
