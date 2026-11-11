---
status: current
last_verified: 2026-11-10
---

# Developer tools in the running game

Developer tools inspect runtime state or request simulation changes through
explicit commands. They are not a second gameplay authority.

## Controls and ownership

[Developer hotkeys](developer-hotkeys.md) lists the current bindings: `F1`
toggles debug overlays, `Shift+F1` selects combat geometry, `F3` opens the
resource inspector, and `F8` writes a gameplay trace.

- `ambition_dev_tools` owns debug switches, tool state, and reusable adapters.
- Runtime/provider systems own queries and command handlers that inspect or
  change authoritative state.
- App/render crates present read models, geometry, or inspector windows.
- Headless tools should consume the same observations when practical.

## Authoritative geometry observations

The current observation interface is `ambition_geometry_observation`.
Owners register one-shot read-only providers using
`GeometryObserverAppExt::add_geometry_observer`; consumers call `observe_all`
with an `ObservationQuery` containing an optional room and subject.

The collision-world adapter in
`ambition_platformer2d_world/src/geometry_observers.rs` exposes:

| Provider | Meaning |
| --- | --- |
| `collision.authored` | Authored blocks; notes which are absent from the current composed walls |
| `collision.composed` | Current room walls, including platform/feature contributions |
| `collision.for_subject` | Geometry the selected body actually meets, including its gates and portal cuts |

The subject-specific provider returns `needs_subject` when no subject is
supplied. Results use room-pixel coordinates, y downward, and carry an
`active` or `inactive` state with a reason. `to_json` provides the common
headless representation. The provider reads the collision authority rather
than rebuilding another approximation for display.

## Adding a tool

Expose a typed simulation request for an operation that changes the world, or
register a read-only observation for inspection. Prefer stable content/provider
identities for references that must survive reconstruction. Keep heavyweight
rendering/debug work disabled when the tools are not requested.

Use [profiling](../recipes/profiling.md) for performance investigation and
[explaining a tick](../recipes/explaining-a-tick.md) for causal debugging.
