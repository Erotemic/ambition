---
status: current
last_verified: 2026-11-16
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
| `collision.authored` | Authored geometry; notes which pieces are absent from the current composed walls |
| `collision.composed` | Current room walls, including platform/feature contributions (a ridden mount's back and slope) |
| `collision.for_subject` | Geometry the selected body actually meets, including its gates and portal cuts |

Each of the three lists every kind of geometry an `ae::World` holds
(`world_pieces`): blocks (`solid`, `one_way`, ...), lines (`line` for a
standing surface, `solid_line` for a wall, as polylines; with `carried`,
`moving`, `turning`, `drawn_by_art` when they apply), water (`water`,
`clear`/`murky`) and climbable regions (`climbable`, `ladder`/
`climbable_wall`/`vine`). A slope such as Full Stack's guitar or the back of
GNU-ton's gnu is a `line` + `carried` polyline in `collision.composed`.

The subject-specific provider returns `needs_subject` when no subject is
supplied. Results use room-pixel coordinates, y downward, and carry an
`active` or `inactive` state with a reason. `to_json` provides the common
headless representation. The provider reads the collision authority rather
than rebuilding another approximation for display.

## Making new geometry visible

New geometry is visible in F1 and in `inspect_geometry` by construction, at
one of two places:

1. **A new kind of collision geometry on `ae::World`** (a new field beside
   `blocks`, `chains`, `water_regions`, `climbable_regions`): `world_pieces`
   in `ambition_platformer2d_world/src/geometry_observers.rs` destructures
   `ae::World` with no `..`, so the build fails until the field is listed
   there (as pieces with roles) or named there as not geometry a body meets.
   All three collision observers then show it.
2. **Geometry that is not on the world** (combat volumes, interaction reach,
   portal openings, a new domain): its owner registers an observer with
   `app.add_geometry_observer(ObserverInfo { name, description }, system)`,
   a read-only system `In<ObservationQuery> -> Vec<GeometryObservation>`.
   It joins the catalog, F1's Geometry Observer row, any preset that names its
   namespace and `inspect_geometry --provider` with no edit elsewhere. Add
   its name to the shipped-catalog list in
   `game/ambition_app/tests/observed_geometry.rs` so a composition that drops
   it goes red.

A new primitive goes on `ObservedShape` (`Aabb`, `Segment`, `Polyline`,
`Circle`, `Polygon`); every match on it is exhaustive, so F1's outline and
label code will not compile until they draw it. A role without a colour in
`known_role_color` draws in the fallback colour and the legend names it.

Check a room headless:

```sh
cargo run -p ambition_app_tools --bin inspect_geometry -- \
    --room boss_main_stage --provider collision.composed --ticks 30 --no-subject --out geometry.json
```

## Adding a tool

Expose a typed simulation request for an operation that changes the world, or
register a read-only observation for inspection. Prefer stable content/provider
identities for references that must survive reconstruction. Keep heavyweight
rendering/debug work disabled when the tools are not requested.

Use [profiling](../recipes/profiling.md) for performance investigation and
[explaining a tick](../recipes/explaining-a-tick.md) for causal debugging.
