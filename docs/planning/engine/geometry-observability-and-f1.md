# Geometry observability — gameplay truth in F1 and headless inspection

**State:** Final verification. GOBS-0 through GOBS-6 are implemented.
**Owner:** [inspection and diagnostics](inspection-diagnostics-and-workbench.md).
The current contract and commands are in
[developer tools](../../systems/developer-tools.md).

## Result

F1 and headless tools read domain providers through one geometry report.
`ObservedGeometryPlugin` installs the collector and renderer in Ambition and
in the generic F1 host. The visible app test presses F1 and steps the
production schedule.
The report names the session, live room instance, subject and simulation tick.
It contains source primitives, roles, state and refusal reasons. Portal parts
retain their source cuts, affine maps and paths. Circles remain analytic in
JSON; display outlines alone use line segments.

The Authoring and Collision presets use resolved collision observations.
They do not overlay authored walls on a subject's open gate or portal cut.
Each view uses its own subject and render mask. The Developer page can select
another live body, an installed provider, a domain preset, or the comparison
between tick and presented body positions. Invalid contexts and unavailable
providers produce visible diagnostics. Disabled collection returns before
it scans bodies or calls providers.

## Producer coverage

| Authority | Provider | Supported geometry and state |
| --- | --- | --- |
| `CollisionWorld` | `collision.authored`, `collision.composed`, `collision.for_subject` | Blocks, moving platforms, feature solids, gate permissions, portal cuts, slopes, mounts, water and climbable regions |
| Movement kernel | `body.sweep` | Last swept box, path and ground/head/wall contact flags |
| `BodyGeometry` | `body.material` | Native body parts with exact source cuts and maps; query refusals are inactive |
| `CombatGeometryView` | `combat.volumes` | Published/fallback/intangible hurt volumes and live strikes, including portal parts and geometry refusals |
| Projectile material and burst data | `combat.projectiles` | Live native parts; impact-burst previews are inactive and labelled |
| `InteractReach` and the shared target choice | `interaction.reach` | Reach, facing, talk/switch/chest candidates, selected target, doors and rejection reasons |
| Portal placement and room pairing | `portal.apertures` | Turned openings; unpaired or disabled ends are inactive |
| Navigation advisor | `navigation.graph` | Actual graphs used this tick, surfaces, hops, drops, routes and goals; selected-body filtering |
| Live room specification | `world.regions` | Authored loading, camera, gravity and encounter regions, and lock-wall regions |

Authored regions show where a rule applies. They do not claim that the rule
has fired. `collision.for_subject` is the resolved world query;
`body.material` shows native parts used by material contacts. `body.sweep`
shows the kernel's recorded box and contact flags, not an invented contact
point. A dormant attack has no live strike. A refused material query has no
active fallback box.

## Verification and review

```sh
cargo test -p ambition_app --test app_it -- observed_geometry
cargo test -p ambition_geometry_observation -p ambition_sim_view -p ambition_render -p ambition_dev_tools --lib
cargo check -p ambition_app -p ambition_app_tools --bins
```

The app lab compares F1 and headless output, keeps the portal lab live while
another body crosses to the hub, checks two views in one room, and refuses a
foreign subject. Its combat case observes a miss followed by a real hit.
The view tests check an analytic scaled portal cap and its exact source
planes. Render tests check the cap's display outline and unavailable-provider
labels. The shipped-catalog test detects an omitted supported provider.

Use `inspect_geometry --cost` for provider time, piece counts and JSON size.
Use `capture_scene --geometry-overlay A,B,prefix.` for a review image. Full
commands are in the system document. Current measurements are recorded after
the final verification run.

## Standing rules

- Domain owners supply observations from their actual queries or read models.
  The observer does not create another physics authority.
- New `ae::World` fields must be handled by the exhaustive `world_pieces`.
  Other domains register `add_geometry_observer` beside their capability.
- New providers join F1 and headless discovery without a game draw switch.
  Unknown roles remain visible and are named in the legend.
- Geometry stays at the simulation tick pose. Presentation comparison is
  explicit. A room view and a subject view are distinct queries.
- No render adapter is required to produce a geometry report.
- No diagnostic geometry is built while collection is disabled.

Broad spatial semantics, runtime geometry editing and an editor scene tree
remain outside this task.
