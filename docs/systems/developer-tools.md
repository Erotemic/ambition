# Bevy developer tools

The sandbox now uses two Bevy-native debugging layers:

1. **Bevy Gizmos** for immediate-mode world overlays.
2. **bevy-inspector-egui** for reflected live-editing of tuning resources.

This is deliberately a developer/debug workflow, not an in-game editor. It lets
us tune feel and inspect ECS state before spending time on custom editor UI.

## Cargo feature notes

`dev_tools` enables the inspector but **not** the Bevy asset file watcher.
The watcher (Bevy 0.18's `notify`-backed asset hot-reload) calls
`inotify_init()` to obtain an inotify *instance* — and the scarce Linux
resource here is `/proc/sys/fs/inotify/max_user_instances`, which
defaults to **128** on most kernels and is shared across every program
the user is running (VSCode language servers, file managers, sync
clients, browser tabs, dev servers, etc.). When Bevy tries to grab an
instance and the user is already near the cap, `inotify_init()` returns
`EMFILE`, which the notify crate surfaces as `Failed to create file
watcher … "Too many open files"`. The per-file watch count
(`max_user_watches`, default tens of thousands) is essentially never the
issue.

Asset hot reload is split into its own feature, `dev_hot_reload`. Enable
it only when you are actually iterating on textures/fonts/spritesheets:

```bash
cargo run -p ambition_gameplay_core --bin ambition_gameplay_core --features dev_hot_reload
```

If you DO want hot reload but still hit `EMFILE`, raise the per-user
instance cap (and optionally the watch cap) on your host:

```bash
# Linux inotify instance quota — default 128, shared across all programs
# the user is running. Bump it; the kernel just allocates a bit more
# slab memory.
echo 1024   | sudo tee /proc/sys/fs/inotify/max_user_instances
# Watch count: usually fine, but bumping costs nothing.
echo 524288 | sudo tee /proc/sys/fs/inotify/max_user_watches

# Per-shell open-fd limit. Generally not the cause of `inotify_init`
# EMFILE specifically, but a low ulimit -n can trip other code paths.
ulimit -n 65536
```

Persist via `sysctl.d` for inotify and `/etc/security/limits.d/` for
ulimits if you want it to survive reboot.

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

The inspector exposes sandbox-side mirrors instead of making reusable mechanics
state reflection-driven.

- `DeveloperTools`: HUD, inspector, debug view mode, debug art mode, and the
  advanced per-overlay toggles used by the Custom view.
- `EditableAbilitySet`: live ability flags such as jump, wall climb, dash, blink,
  fly, attack, pogo, and rebound.
- `EditableMovementTuning`: live movement parameters copied from the RON manifest
  at startup, including gravity, speed, friction, dash, blink, wall, flight, and
  pogo values.
- `SandboxFeelTuning`: sandbox-only timing values such as bullet-time scale,
  time-ramp rates, double-tap windows, hitstop, reset flash, and room-transition
  cooldowns.

The editable movement and ability resources are intentionally sandbox-side data
mirrors. The actual simulation still receives plain mechanics data from
`engine_core`, keeping reflection and inspector policy out of hot simulation
code.

## Gizmo overlays

The debug overlay is organized around named debug views first, with individual
overlay booleans treated as advanced/custom state:

- `Gameplay` keeps the game view clean.
- `Authoring` shows room/world context plus player, blink, combat, feature,
  health, moving-platform, and rebound overlays.
- `Collision` hides art and fills collision/player/feature/platform volumes.
- `Triggers` focuses loading zones, camera frames, player overlap, and feature
  trigger volumes.
- `Combat` focuses player vectors, combat previews, actors, projectiles, and HP.
- `All` enables every lightweight overlay.
- `Custom` preserves hand-edited toggle combinations.

`Debug Art` is separate from the view mode and can be `normal`, `placeholder`,
or `hidden`. View presets choose a recommended art mode, but the art mode can be
changed afterward without changing which debug data is drawn.

The available overlay data includes:

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
source is still `crates/ambition_gameplay_core/assets/ambition/sandbox.ron`. Once a
tuned value feels good, copy it back to the RON manifest or promote it to a more
formal editor/save workflow later.

Implementation note: `ResourceInspectorPlugin` and `WorldInspectorPlugin` require `EguiPlugin::default()` to be added first. The visible-app builder in `crates/ambition_gameplay_core/src/app/mod.rs` (`add_presentation_plugins`) registers `EguiPlugin` immediately after `DefaultPlugins` so the inspector quick plugins can initialize safely.

## Other dev hotkeys (cross-references)

These hotkeys live in their own modules but are listed here for discoverability. Their full documentation is in the linked module:

- `F8` — gameplay trace recorder dump (`crate::trace::handle_trace_hotkey`).
- `F11` — manual LDtk hot-reload.
- `F12` — toggle LDtk auto-reload polling.
- `F6` / `F7` — display mode hotkeys (windowed / borderless); see `docs/systems/display-modes.md`.
