# Developer hotkeys

`crates/ambition_platformer2d_shared_tangle/src/developer_hotkeys.rs` owns the
physical bindings. It maps exact key chords to `DeveloperAction`; consumers
match the semantic action rather than reading function keys independently.

## Default bindings

| Chord | Action |
|---|---|
| `F1` | Toggle debug overlay |
| `Shift+F1` | Toggle combat overlay |
| `F2` | Toggle developer slow motion |
| `F3` | Toggle resource inspector |
| `F4` | Toggle world inspector |
| `F5` | Toggle overview camera |
| `F6` | Toggle FPS overlay |
| `Shift+F6` | Toggle and reset gamepad-axis probe |
| `F7` | Toggle portal gun |
| `F8` | Dump a gameplay trace |
| `Shift+F8` | Dump the portal view cone |
| `F9` | Request bounded GGRS rollback proof |
| `F10` | Quit session to the home route |
| `F11` | Validate/apply pending LDtk content |
| `F12` | Toggle LDtk auto-apply |

These bindings are defaults, not an additional gameplay input layer. Add or
change chords in `DeveloperHotkeyBindings::default`; the plugin validates
uniqueness. Exact matching prevents a modified chord from also firing its
unmodified counterpart. File-writing diagnostics run outside rollback.

Display mode and portal mapping convention are separate configuration choices;
they have no developer shortcuts.

## Combat view (`Shift+F1`)

The combat preset shows hurt regions, active hitboxes, body collision, guards,
projectile state, and damage labels. The visual implementation is in
`game/ambition_app/src/dev/debug_overlay/gizmos.rs`. Use it to inspect actual
combat volumes; for offscreen screenshots, `capture_scene --combat-overlay`
enables the same class of evidence.

## Gamepad-axis probe (`Shift+F6`)

The probe samples the raw Bevy `Gamepad` axes by connected controller and holds
observed peaks. Toggle it to reset peak values; toggle it off to log a readout
under `ambition::gamepad_probe`.

It exists to diagnose hardware that reports a smaller-than-expected stick
range. Smash flick detection applies `AttackGestureTuning::flick_threshold`
*after* input deadzone processing, so a controller may navigate normally yet
never cross the attack threshold. The probe reports the effective threshold
using the active settings rather than requiring an agent to guess a new tuning
constant. See `game/ambition_app/src/dev/gamepad_probe.rs` and
`crates/ambition_characters/src/actor/attack_gesture.rs`.

The probe diagnoses input; calibration or control-policy changes belong to the
shared input system, not in a fighter-specific workaround.
