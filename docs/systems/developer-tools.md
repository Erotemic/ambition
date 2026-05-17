# Bevy developer tools

The sandbox now uses two Bevy-native debugging layers:

1. **Bevy Gizmos** for immediate-mode world overlays.
2. **bevy-inspector-egui** for reflected live-editing of tuning resources.

This is deliberately a developer/debug workflow, not an in-game editor. It lets
us tune feel and inspect ECS state before spending time on custom editor UI.

## Hotkeys

| Input | Effect |
|---|---|
| `F1` | Toggle the existing debug HUD and gizmo layer. |
| `F2` | Toggle sandbox slow motion. |
| `F3` | Toggle reflected resource inspector windows. |
| `F4` | Toggle the heavier full-world inspector. |

The resource inspector is visible by default because early Ambition work is
mostly movement-feel iteration. The world inspector starts hidden because it is
more intrusive and can be noisier.

## Reflected resources

The inspector exposes sandbox-side mirrors instead of adding Bevy reflection as
a dependency of `ambition_engine`.

- `DeveloperTools`: HUD, gizmo, room, loading-zone, player-vector, blink-preview,
  dummy, platform, and world-inspector toggles.
- `EditableAbilitySet`: live ability flags such as jump, wall climb, dash, blink,
  fly, attack, pogo, and rebound.
- `EditableMovementTuning`: live movement parameters copied from the RON manifest
  at startup, including gravity, speed, friction, dash, blink, wall, flight, and
  pogo values.
- `SandboxFeelTuning`: sandbox-only timing values such as bullet-time scale,
  time-ramp rates, double-tap windows, hitstop, reset flash, and room-transition
  cooldowns.

The editable movement and ability resources are intentionally sandbox-side data
mirrors. The actual simulation still receives plain `ambition_engine` structs so
the reusable engine remains backend-neutral.

## Gizmo overlays

The debug overlay now uses reflected `DeveloperTools` toggles to control:

- room bounds
- room blocks
- loading-zone rectangles
- player body, velocity, facing, ground/wall contact vectors
- blink preview target and blocked desired target
- held attack/pogo hitbox preview
- moving platform AABB
- dummy AABBs and finite dummy HP bars
- rebound block impulse vectors

This should be the first place to add new temporary visualizations while we are
still discovering the right feel for Ambition.

## Notes

Changes made through the inspector are runtime-only. The current persistent data
source is still `crates/ambition_sandbox/assets/ambition/sandbox.ron`. Once a
tuned value feels good, copy it back to the RON manifest or promote it to a more
formal editor/save workflow later.

Implementation note: `ResourceInspectorPlugin` and `WorldInspectorPlugin` require `EguiPlugin::default()` to be added first. The visible-app builder in `crates/ambition_sandbox/src/app.rs` (`add_presentation_plugins`) registers `EguiPlugin` immediately after `DefaultPlugins` so the inspector quick plugins can initialize safely.

## Other dev hotkeys (cross-references)

These hotkeys live in their own modules but are listed here for discoverability. Their full documentation is in the linked module:

- `F8` — gameplay trace recorder dump (`crate::trace::handle_trace_hotkey`).
- `F11` — manual LDtk hot-reload.
- `F12` — toggle LDtk auto-reload polling.
- `F6` / `F7` — display mode hotkeys (windowed / borderless); see `docs/systems/display-modes.md`.
