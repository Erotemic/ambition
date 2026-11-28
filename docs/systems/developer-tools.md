---
status: current
last_verified: 2026-11-17
---

# Developer tools in the running game

Developer tools inspect runtime state or request simulation changes through
explicit commands. They are not a second gameplay authority.

The developer menu has **Unlock all abilities**, enabled by default. It lifts
authored room and equipment access limits for each controlled body's own kit.
It does not replace that body's kit. Physical limits, portal transit guards,
and individual developer ability switches still apply. The setting persists
and uses the normal mechanical-edit admission path.

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
The render host installs `ObservedGeometryPlugin` for scheduled collection
and view-scoped drawing. F1 and the CLI use
`ambition_sim_view::geometry_inspection::inspect_geometry`
to validate that context. A foreign subject or a retired room is refused
before any provider runs. The report records the session, room instance,
subject identity and simulation tick. Its pose is the simulation tick pose;
**Tick vs Drawn** shows the separate presentation comparison.

The collision-world adapter in
`ambition_platformer2d_world/src/geometry_observers.rs` exposes:

| Provider | Meaning |
| --- | --- |
| `collision.authored` | Authored geometry; notes which pieces are absent from the current composed walls |
| `collision.composed` | Current room walls, including platform/feature contributions (a ridden mount's back and slope) |
| `collision.for_subject` | Resolved world queries for the selected body, including gates and portal cuts |
| `body.material` | Native body parts, source cuts, portal paths and exact maps |
| `body.sweep` | The last movement sweep and its contact flags |
| `body.regions` | The selected body's admitted parts and each water and climbable region they touch: the one movement reads, and each touch it drops with why |
| `combat.volumes` | Published or fallback hurt volumes, intangible bodies and live strikes |
| `combat.projectiles` | Live projectile parts and inactive impact-burst previews |
| `interaction.reach` | Reach (a cut body's admitted parts), facing, candidates, the chosen target, doors and refusal reasons |
| `portal.apertures` | Openings; an unpaired end is inactive with its reason |
| `navigation.graph` | The selected body's advised graph and route; all advised graphs in the room view |
| `world.regions` | Authored loading, camera, gravity and encounter regions |

Each of the three lists every kind of geometry an `ae::World` holds
(`world_pieces`): blocks (`solid`, `one_way`, ...), lines (`line` for a
standing surface, `solid_line` for a wall, as polylines; with `carried`,
`moving`, `turning`, `drawn_by_art` when they apply), water (`water`,
`clear`/`murky`) and climbable regions (`climbable`, `ladder`/
`climbable_wall`/`vine`). A slope such as Full Stack's guitar or the back of
GNU-ton's gnu is a `line` + `carried` polyline in `collision.composed`.

The subject-specific provider returns `needs_subject` when no subject is
supplied. Results use room-pixel coordinates, y downward, and carry an
`active` or `inactive` state with a reason. `GeometryReport::to_json` writes
`ambition.geometry_observation.v2`. A clipped shape retains its exact source
primitive, source half-planes, affine map and portal path. A cut circle stays
a circle in the report. Only its screen outline uses line segments. The provider reads the collision authority rather
than rebuilding another approximation for display.

## Select a view and body

F1 uses each local view's subject. Geometry and labels use that view's render
mask, including when two views show one room. The Developer page provides:

- **View Mode:** Authoring and Collision use resolved collision geometry.
  Combat, Triggers and Navigation select their domain providers. All selects
  every provider.
- **Geometry Observer:** select one installed provider, or the preset.
  One provider also shows piece labels and inactive reasons.
- **Geometry Subject:** select a live body by its stable simulation identity,
  follow each view's subject, or inspect the room without a subject.
  Navigation starts in the room view, so it shows all graphs in use. A subject outside a view's room produces a
  visible refusal in that view.
- **Tick vs Drawn:** compare mechanical and presented body positions.

The legend distinguishes an empty answer, a missing subject and an unavailable
provider. An unknown role uses a fallback colour and is named in the legend.
F1 is off during normal play. With the layer off, the collector clears its
reports and returns before it scans bodies or calls any provider.

## Headless reports and captures

Build the tools once, then use the binaries:

```sh
cargo build -p ambition_app_tools --features game --bin inspect_geometry --bin capture_scene
target/debug/inspect_geometry --list
target/debug/inspect_geometry --room portal_lab --subjects
target/debug/inspect_geometry --room portal_lab --provider collision.,body.,combat.,portal. --ticks 30 --out /tmp/geometry.json
target/debug/inspect_geometry --room central_hub_complex --provider navigation. --no-subject --cost
target/debug/capture_scene basement_breakables player /tmp/geometry.png 1280x720 --warmup 30 --geometry-overlay collision.for_subject,body.material
```

Use `--subject SIM_ID` with an identity from `--subjects`. `--no-subject`
asks for the room view. Without either flag, the CLI uses Player 1. `--out`
writes JSON alone; stdout can also contain host log lines. `--cost` reports
21 samples per provider and the complete report's size. The report has no
shape limit or silent truncation.

`capture_scene --geometry-overlay A,B,prefix.` selects the same providers;
`all` selects every provider. `--geometry-subject view|room|SIM_ID` sets its query context.

The Combat preset includes effective collision surfaces and body geometry.
This includes carried surfaces such as a mount's guitar, even when its art
draws the surface. A saved preset uses its current provider selection after
load. Manual layer edits use the Custom mode.
Captures are presentation evidence. The structured report is the geometry
record. Inactive splash previews predict the burst shape at the shot's current
position; the live impact strike appears in `combat.volumes` after impact.

Run the end-to-end lab with:

```sh
cargo test -p ambition_app --test app_it -- observed_geometry
```

It compares F1 with headless reports, crosses from the portal lab while a
second body keeps it live, checks separate view subjects and foreign-subject
refusal, and observes a missed swing followed by a real sandbag hit.
The geometry and view crates also test analytic portal caps, display clipping,
provider availability and default-off collection.

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
   `app.add_geometry_observer(ObserverInfo { name, description, needs_subject }, system)`,
   a read-only system `In<ObservationQuery> -> Vec<GeometryObservation>`.
   It joins the catalog, F1's Geometry Observer row, any preset that names its
   namespace and `inspect_geometry --provider` with no edit elsewhere. Add
   its name to the shipped-catalog list in
   `game/ambition_app/tests/observed_geometry.rs` so a composition that drops
   it goes red.

A new primitive goes on `ObservedShape` (`Aabb`, `Segment`, `Polyline`,
`Circle`, `Polygon`, `Clipped`); every match on it is exhaustive, so F1's outline and
label code will not compile until they draw it. A role without a colour in
`known_role_color` draws in the fallback colour and the legend names it.

Check a room headless:

```sh
cargo run -p ambition_app_tools --features game --bin inspect_geometry -- \
    --room boss_main_stage --provider collision.composed --ticks 30 --no-subject --out geometry.json
```

## Adding a tool

Expose a typed simulation request for an operation that changes the world, or
register a read-only observation for inspection. Prefer stable content/provider
identities for references that must survive reconstruction. Keep heavyweight
rendering/debug work disabled when the tools are not requested.

Use [profiling](../recipes/profiling.md) for performance investigation and
[explaining a tick](../recipes/explaining-a-tick.md) for causal debugging.
