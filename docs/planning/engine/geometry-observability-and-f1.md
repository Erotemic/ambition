# Geometry observability — gameplay truth in F1 and headless inspection

**State:** SELECTED through [`GEOMETRY-OBSERVABILITY`](../queue.md#geometry-observability--render-the-effective-geometry-behind-gameplay). This is a bounded debugging/authoring capability, not a redesign of physics or a new simulation authority.

**Owner relationship:** [`inspection-diagnostics-and-workbench.md`](inspection-diagnostics-and-workbench.md) owns composed, read-only inspection and why-not diagnostics. This document owns the *geometry-observation contract* and the first set of supported geometric producers. [`world-geometry-and-spatial-semantics.md`](world-geometry-and-spatial-semantics.md) remains deferred for broad spatial-semantic unification. [`moveset-inspector.md`](../moveset-inspector.md) owns art/contact alignment tooling.

## Product promise

When F1 is enabled, a developer can see the meaningful shapes and boundaries that the current simulation uses to decide whether something **blocks, supports, hits, hurts, senses, triggers, can be interacted with, or can be traversed**. When the answer depends on an actor or query context, the developer can select that context and see both the effective shape and *why* it differs from the authored/base one. The same observations are machine-readable in a headless test/tool.

Do not claim that every shape always appears in a single cluttered view. All relevant producers must be discoverable, and selecting an appropriate mode/subject must give a truthful, inspectable answer. An absent shape must be distinguishable from an inactive shape, a filtered/denied shape, and a geometry producer that has not installed its diagnostic adapter.

### Godot comparison: parity target, not an architecture copy

Godot has `Debug > Visible Collision Shapes` for running scenes, including configurable debug colors and outlines (helpful for overlapping collision/hit/interaction shapes). `Debug > Visible Navigation` shows navigation geometry and links. The Remote Scene dock allows inspection of live scene nodes. These give users a predictable debugging vocabulary and live spatial context.

The cautionary counterexample is instructive: Godot's documented navigation visualizers are based on `SceneTree` nodes, and navigation changes made exclusively through `NavigationServer` APIs may not appear in them. Ambition should **not** recreate a visualizer that is accurate only for the authoring representation while gameplay queries something else.

Official references (consulted 2026-11-07):

- [Godot: Collision shapes (2D), the node/resource model](https://docs.godotengine.org/en/stable/tutorials/physics/collision_shapes_2d.html)
- [Godot: Navigation debug tools](https://docs.godotengine.org/en/stable/tutorials/navigation/navigation_debug_tools.html)
- [Godot: Overview of debugging tools / Remote Scene dock](https://docs.godotengine.org/en/stable/tutorials/scripting/debug/overview_of_debugging_tools.html)

Ambition parity means **predictable visibility of active physical and query shapes, categories and colors, navigation, and live-entity inspection**. Ambition's stronger requirement is effective subject-scoped geometry and a no-GUI diagnostic export. It does **not** mean adopting Godot's scene tree, physics engine, or mandatory editor process.

## Evidence from the current implementation

This is an integration and coverage problem, not a blank-slate gizmo renderer.

| Existing source | Already provides | Gap to close |
| --- | --- | --- |
| `crates/ambition_render/src/rendering/debug_viz.rs` | Shared F1 renderer, primitives, room blocks, body/feature/combat layers; `DebugVizPlugin` | `draw_world_blocks` iterates `world.blocks`, while actual effective collisions may be dynamically composed and subject-specific |
| `game/ambition_app/src/dev/debug_overlay.rs` and `debug_overlay/gizmos.rs` | Game-specific layers for portals, projectiles, loading zones, bosses and labels | New domains usually require explicit hand-written draw integration; some effective regions have no layer |
| `crates/ambition_dev_tools/src/dev_tools/profiles.rs` and `developer_tools.rs` | Named Gameplay/Authoring/Collision/Triggers/Combat/All/Custom presets and per-layer switches | A preset only exposes producers that were explicitly wired; F1 does not prove coverage |
| `crates/ambition_render/src/rendering/retained_drawings.rs` | `DrawingScope::Room` / `DrawingScope::View`, retained gizmo lifetime | Useful output backend; do not replace it with permanent debug entities per shape |
| `crates/ambition_platformer2d_world/src/collision.rs` | `CollisionWorld`, composited room solids and `solids_for(subject)` | Need read-only visualization of the *same resolved query*, including portal carves and subject-specific gate permissions |
| `crates/ambition_sim_view/src/combat_geometry_view.rs` | `CombatGeometryView` with effective `DamageableVolumes`, fallback/intangible status, live strikes and portal-cut pieces | Register it as one geometry-observation source rather than duplicate hitbox calculations |
| `crates/ambition_sim_view/src/affordances/interactable_proximity.rs` and `ambition_platformer2d_interaction::InteractReach` | Real interaction eligibility rules and priority | Missing complete reach/facing/door-preemption visualization and rejection reasons |
| `game/ambition_app/src/dev/navigation_overlay.rs` | Navigation graph, hop/drop links and advice | Incorporate through common selection, live-room scoping and headless observation path |

Verify these baselines against the current checkout before changing APIs; the source snapshot informing this plan is dated 2026-11-07.

## Contract: a read-only geometric observation, not another physics truth

A domain producing spatial gameplay semantics provides a read-only observation adapter over its **actual resolved geometry** or already-authored authoritative data. The observer may project/copy it, but must not recompute alternative collision rules or mutate simulation state. No global enum needs to enumerate every gameplay domain; capability/domain providers contribute namespaced descriptors through the existing composition/discovery pattern.

Minimal shared descriptor fields (exact Rust types selected in GOBS-1):

- **Provenance:** namespace/provider, source entity or stable authored ID, authored/prepared origin when available, live room instance, and stable observation key.
- **Role:** `solid`, `one_way`, `surface`, `hit`, `hurt`, `interact`, `trigger`, `sensor`, `projectile`, `portal`, `navigation` or a domain-specific string; multiple roles are allowed.
- **Geometry:** point/segment/ray, AABB, OBB, circle/capsule, polygon, half-plane/clipped region or a collection of actual primitives. Include coordinate frame and applied transform; use canonical geometry when available, never replace exact production geometry with a guessed rectangle without labeling it as an approximation.
- **State:** available/active, disabled, intangible, query-filtered, speculative/presented; the *reason* for omission or refusal if supplied by the owning domain. Do not silently draw filtered geometry as collidable.
- **Query:** selected body/entity, collision mask/layer/faction/room, one-way direction, facing, range or other relevant conditions, and optionally the precise contact/raycast/interact attempt and outcome. Base geometry and effective-for-subject geometry are different views.
- **Time:** simulation/session frame or authoritative generation, plus presentation interpolation clock when displayed geometry uses it. Distinguish mechanical tick position from interpolated visual position; never silently combine them.

Use a deterministic, versioned headless representation (e.g. compact JSON with stable ordering and explicit unit/frame conventions) and a shared observation read model consumed by both CLI and F1. Keep screen color, linewidth and alpha in the presentation adapter, **not** authoritative spatial data. Numeric precision and full geometry should survive serialization independently of display resolution.

**Registration:** a new installed capability exposes an observation provider alongside the geometry it owns, through the engine's existing capability-composition seam. Its provider is automatically discoverable in the F1 layer list and headless `describe/inspect` output; no edit to `ambition_app::dev::debug_overlay::draw_debug_overlay` or a central match over gameplay-domain names is needed. Where an existing domain has a common component/read model, a reusable adapter can register it once. Arbitrary novel custom collision code cannot be introspected magically: it must publish a provider or explicitly declare that no inspectable geometry exists. Tests enforce that declared supported domains cannot accidentally omit the provider.

**Performance:** when disabled, do not walk all entity colliders or build diagnostic polygons per frame. On demand, scope by active camera/live room, subject, role and viewport, and reuse `RetainedDrawings`; introduce budgets and truncation indicators for dense scenes. Do not add debug-only work to the deterministic rollback state, host authority, normal release hot path or content publication. Observation has well-defined stale/unavailable states when a room is not live.

## Ordered implementation packets

### GOBS-0 — coverage census and a reproducible missing-outline lab

1. Enumerate each geometry producer and its *actual query API*, not only each existing gizmo drawer: authored solids, per-subject solids, moving platforms, one-way surfaces, hit/hurt volumes, attacks/projectiles, interactables, camera/loading triggers, portal aperture/partition geometry, and navigation surfaces/links.
2. For each, record current render adapter, F1 mode, relevant room/view/subject, known invisibility condition, and a minimal fixture. Identify geometry that is not currently in any debug view.
3. Build a small two-live-room lab with a gate/portal carve, moving platform, melee attacker, NPC talk target, item/switch/door, projectile, loading zone and navigation link. Use a second controlled body to show subject-dependent collision truth. Reuse existing fixtures where possible.

**Done when:** a checked-in producer-to-observer matrix explicitly distinguishes covered, missing, conditional and out-of-scope domains, and one executable lab demonstrates a missing or misleading gizmo before the fix. Do not claim source-code coverage implies user-visible coverage.

### GOBS-0 result (2026-11-10): census and the first lab

**Lab:** `game/ambition_app/tests/f1_blocks_against_collision.rs`,
`probe_f1_blocks_against_the_walls_a_body_meets` (ignored; prints). In
`basement_breakables` it compares what F1's blocks layer draws
(`RoomGeometry`, through `draw_world_blocks`) with what a body's sweep meets
(`CollisionWorld::room(..).solids()`). The blocks layer draws 8 solids; a body
meets 18. The 10 missing ones are five breakable solids (`ecs-breakable …`,
Barrier or OneWay) and five world pogo orbs (`ecs-pogo-target …`). The blocks
layer outlines nothing that is not there.

**A gameplay defect the lab found:** a platform a body stands on to crumble
also lends the world a pogo orb (`PogoTargetContributor`'s fallback). Broken,
its solid went and its orb stayed, so a body could pogo off the air where it
had been. Fixed in `rebuild_feature_ecs_world_overlay`; witness
`a_broken_platform_leaves_no_pogo_orb_behind`.

**Producer → observer matrix.** Read from the code on 2026-11-10. "Lab" means
measured by the probe above; the other rows are read, not measured.

| Producer (what gameplay queries) | Where the truth is | F1 layer (toggle) | Status |
|---|---|---|---|
| Authored room blocks | `RoomGeometry` → `CollisionWorld` base | world blocks (`show_world_blocks`) | Covered (lab) |
| Moving platforms | `MovingPlatformSet`, composed in `solids()` | moving platforms (`show_moving_platform`) | Covered, on its own toggle, not in the blocks layer |
| Breakable solids (`ecs-breakable`) | feature overlay `blocks`, composed in `solids()` | feature boxes (`show_feature_hitboxes`), labelled "breakable" | **Misleading:** the feature layer outlines every breakable, broken or whole, and does not show whether it is a Barrier or OneWay; the blocks layer omits them (lab) |
| World pogo orbs (`ecs-pogo-target`, fallback) | feature overlay `blocks` (`PogoOrb`) | none found | **Missing** (lab) |
| Gate and lock walls, and the bodies a gate passes | overlay `gate_solids`, `gate_passes` → `solids_for(subject)` | none; the body probes read `solids_for(entity)` but nothing draws the wall or its pass list | **Missing** (read) |
| Body barriers | overlay `body_barriers` → `barriers_for(subject)` | none found | **Missing** (read) |
| Portal holes and glue, per body | overlay `portal_holes`, `portal_glues` → `solids_for(subject)` | portal panes (`draw_portals`, feature toggle) | **Conditional:** the pane is drawn; the hole a body has open is not |
| Removed authored blocks | overlay `removed_block_names` | blocks layer draws the base | **Unmeasured:** no removal in the lab room; the blocks layer would outline a removed block |
| Water regions, climbable carves | overlay `water_regions`, `climbable_carves` | water draws as art (`rendering/world.rs`); no F1 layer found | **Missing** (read) |
| Surface chains | `World` chains | blocks layer (`draw_surface_chains`) | Covered (read) |
| Hurt and hit volumes, strikes | `CombatGeometryView` | combat view (`draw_combat_geometry_view`) | Covered (read; the owner's own read model) |
| Projectiles | projectile bodies | `draw_projectile_debug` | Covered (read) |
| Interaction reach | `InteractReach`, interactable proximity | Observed Geometry (`interaction.reach`, 2026-11-11) | Covered: reach, candidates, the chosen one and each refusal's reason |
| Loading zones, LDtk spine | room spec zones | `draw_loading_zones` (`show_loading_zones`) | Covered (read) |
| Camera frame | camera snapshot | `draw_camera_frame` (`show_camera_frame`) | Covered (read) |
| Navigation graph and links | navigation graph | `navigation_overlay` (`show_navigation`) | Covered in its own overlay (read) |

**What GOBS-1 should start from:** the two misleading rows (breakables and
removed blocks) and the per-subject rows (gates, barriers, portal holes) are
all differences between `RoomCollision::base()`, `solids()` and
`solids_for(subject)`. An observation provider over those three views would
cover them with no domain-specific drawer.

### GOBS-1 — domain-contributed observation protocol

**First slice landed (2026-11-10).** `crates/ambition_geometry_observation`
(bevy_ecs and bevy_app only) holds the descriptor
(`GeometryObservation`: key, roles, shape, active or inactive with the
owner's reason), the registry (`add_geometry_observer`, a one-shot system
per provider) and one entry point, `observe_all`, with a JSON form
(`to_json`, `ambition.geometry_observation.v1`, room pixels, y down). A
provider that needs a body answers `needs_subject` without one; a provider
whose resources are not in the App answers `unavailable`, not an empty list.
Installed providers:

- `collision.authored`, `collision.composed`, `collision.for_subject`
  (`ambition_platformer2d_world::geometry_observers`): `base()`, `solids()`
  and `solids_for(subject)` of `CollisionWorld`. An authored block the
  composed walls do not hold, and a wall a body does not meet (its open
  gate), are inactive with that reason.
- `combat.volumes` (`ambition_sim_view::combat_geometry_observer`): hurt and
  hit volumes from `CombatGeometryView`; an intangible body is listed
  inactive by its box; a portal-cut body is listed whole with role
  `portal_cut` (its pieces are GOBS-3's).

F1: the Developer toggle **Observed Geometry** (`show_observed_geometry`).
`collect_observed_geometry` asks every provider about each shown room, with
the player in it as the subject, only while the overlay, gizmos and that
toggle are on; `draw_observed_layers` draws the answer (colour by first
role; inactive faint and crossed). Witness:
`game/ambition_app/tests/observed_geometry.rs` (a test's own provider is in
the catalog, F1's feed and the headless answer, with equal geometry; the
layer off asks nothing).

**Headless inspection (2026-11-10):** `inspect_geometry`
(`game/ambition_app_tools`): `--list` prints the catalog; `--room R` boots a
room, steps `--ticks` frames and writes `observe_all`'s JSON (to `--out FILE`
alone; stdout also carries the host's `[live-reload]` lines), for Player 1 as
the subject or the room's own view with `--no-subject`. `--provider` selects
by name or by prefix (`collision.`); an unknown name is refused with the
installed list. In `basement_breakables` the composed walls answer the 18
solids GOBS-0 counted; `collision.for_subject` with no subject answers
`needs_subject`. It is a binary beside `room_census`, not an `agent_query`
command: `agent_query.py` reads the source tree and runs no game.

**Per-provider selection in F1 (2026-11-10):** the Developer page's
**Geometry Observer** row cycles the layer between every observer and each
installed one in turn (`DeveloperTools::observed_geometry_only`). The names
come from the registry: F1's collector publishes them
(`observed_geometry_providers`, not a setting) while the layer is on, so a
new provider joins the cycle with no edit to the menu. Witnessed in
`f1_and_headless_inspection_read_one_observation`: one step draws only the
first observer, a step back draws them all. Presets (several at once, a
prefix like the headless `collision.`) are GOBS-4's.

**Labels (2026-11-10):** with one observer selected on that row, each piece
is labelled at its anchor (a box's top-left corner, a segment's middle, a
circle's centre) with its key, its roles and, when inactive, the owner's
reason (`observation_labels` in `ambition_render`'s `observed_geometry.rs`,
drawn through the overlay's label sink by `label_observed_geometry` in the
app's `debug_overlay.rs`). With every observer drawn there are no labels: they
would bury the room. Unit:
`each_observed_piece_is_labelled_with_its_key_roles_and_reason`; the app
wiring has no test of its own (the label sink is the app crate's, and the one
harness that reads it boots a visible app).

**GOBS-1 is done.** Contact normals and hover detail are GOBS-4's.

1. Build a minimal neutral observation descriptor and query/filter schema (room, view, selected subject, category, time). Start by adapting `CombatGeometryView` and one real `CollisionWorld` query, proving that the interface works for both preexisting snapshots and parameterized queries.
2. Register providers compositionally; derive a provider registry/read-only catalog from installed adapters rather than creating an authoritative ECS-wide geometry database.
3. Provide headless `inspect geometry` capability using the repository's existing agent-query/tool-discovery entry point (inspect current CLI syntax first). Support namespaced role lists, source identity, selected-subject queries, provenance and refusal/absence diagnostics. Wire F1 to the same descriptors.
4. Establish shape renderer coverage for the primitives providers actually emit, including dashed/transparent inactive shapes, contact normals and text labels where they disambiguate.

**Done when:** a test-only third-party provider installs without modifying central F1/game switch statements, appears in both the headless catalog and F1 data feed, and reports exactly the same geometry in both representations. Disabled providers do not render as if active.

### GOBS-2 — effective collision, contacts and query truth

**First slice landed (2026-11-11): `body.sweep`**
(`ambition_platformer2d_world::geometry_observers`). Each body as the
movement kernel last moved it, read from its own `SweepSample`: the box the
sweep tested at the end of the step (`body_box`), the path from where the
step began (`sweep`, when it moved), and an edge of that box for each
contact the kernel reported (`contact_ground` and `contact_head` from
`BodyGroundState`, along the body's `down`; `contact_wall` from
`BodyWallState`). The subject alone when the query names one, else every
body in the room. F1 draws them through the Observed Geometry layer with no
edit to its drawer beyond a colour for each new role. Headless, in
`central_hub_complex` after 120 ticks: 18 bodies, two NPC paths, ground
contacts on Player 1 and the two NPCs; the fifteen items resting on the
floor (`ground_*`, the portal gun) have a sweep record and no ground state,
so they show no contact. Not yet: candidate versus accepted contacts and
their normals and penetration (the kernel keeps flags, not contacts), and
item 3's reasons.

1. Display **authored base** solids, **composited active-room** solids and **selected-body effective** solids as distinct layers or selectable comparisons. Read the same `CollisionWorld`/`RoomCollision::solids_for` composition used for real sweeps. Include moving platforms, feature solids, portals/carves, gate/one-way policies and the actor reference/gravity frame where relevant.
2. Draw the body shape actually used in sweep/contact tests, not only the coarse player pose box. Expose recent sweep/raycast hit/miss results, candidate versus accepted contacts, normal and penetration/separation where produced by the collision owner. Avoid inventing a synthetic contact merely because shapes overlap.
3. Make individual missing/filtered geometry explainable: portal permission denied, one-way rejected from the wrong side, room mismatch, collision mask excluded or object currently disabled. Preserve the owning domain's predicates; don't add duplicate decision logic to the observer.

**Done when:** the two-body lab visibly differs for a portal opening accessible to one body but not the other; toggling between authored and effective views explains a collision missing from the current F1 base-block view. A headless query proves the same difference. Collision behavior is unchanged with the overlay off and on.

### GOBS-3 — combat, interaction, sensors, navigation and portal parts

**Interaction landed (2026-11-11): `interaction.reach`**
(`ambition_sim_view::interaction_reach_observer`, needs a subject). It asks
the rule the press and the prompt ask (`InteractReach`, and the prompt's
`chosen_in_reach`, which now returns the entity the press is for): the
body's touch box, talk box and the point it reaches with (the press is for
the nearest thing to it), then every person, switch and chest in its live
room. One the press could take is active; the one it is for also has the
role `interact_chosen`; any other is inactive with the clause that refuses
it, which the rule itself names (`InteractReach::refuses_talk` and
`refuses_touch`: another live room, out of talking reach, not touched, not
faced; and around them hostile, untouchable, a door nearer takes the press,
opened, falling). Checked in the hub with a temporary probe: beside a person
facing it, it is chosen; facing away, with `RequiresFacing`, it is "not
faced". Headless: `inspect_geometry --provider interaction.`.

1. Adapt the **effective** combat read model: hurt source (`published`, `fallback`, `intangible`), live active hitboxes, projectile body/splash regions and boss-specific strike/contact geometry. Clearly show a dormant strike as dormant or absent rather than inventing a live hitbox.
2. Add `InteractReach` adapter(s) for talk/chest/switch/door/item eligibility, using the real geometry *and* facing/distance/priority/room predicates. Show both eligible shapes and disabled or overridden candidates; explain why a visible NPC is not interactable. Do not treat merely being nearby as eligibility.
3. Incorporate navigation graph, chosen advice, available traversals/hops/drops and agent-size or actor-frame constraints. Keep debug navigation distinct from collision surfaces, like Godot's separate `Visible Navigation` view.
4. Observe loading/trigger/camera regions and portal openings/paths in their appropriate room. The planned **canonical N-part similarity portal** implementation (`portal-parts-similarity-and-chirality.md`) should publish each part's source region, composed transform/scale/parity, clipping boundary and active contact geometry. Adapt existing `VolumePiece` observations now; do not make finishing the full similarity refactor a prerequisite for the first debug releases.

**Done when:** the lab shows a whiff, a landed hit, an intangible hurtbox, an interactable rejected by facing, a navigation hop and a portal-cut strike using actual gameplay geometry. Every one has a corresponding structured record with time/room/owner.

### GOBS-4 — everyday F1 developer experience

1. Keep F1 master visibility and existing `DebugViewMode` presets; make Collision, Combat, Triggers and navigation modes include their newly registered sources by role. Add a discoverable layer list and legend; allow toggling roles and choosing a viewed actor/room without editing Rust. Do not repurpose Shift+F1 or other established controls.
2. Make the default authoring debug mode informative without being impenetrable: appropriate outlines and low-alpha fills, clear color/line conventions, optional labels, contact arrows and hover/selection detail. If all overlays are hidden, say which master toggle/preset suppresses them. Support split-screen and two simultaneously live rooms, preserving `DrawingScope` and presented-camera alignment.
3. Draw the simulation-tick geometry and presented/interpolated geometry separately when their difference matters (e.g. moving platforms, GNU-ton motion, projectile impacts); selectable comparison makes visual/physics disagreement diagnosable. Do not silently shift mechanical shapes to render positions in a physics-truth view.
4. Provide at least one non-GUI review artifact: a deterministic geometry report and optional captured frame with the selected overlays. It must work from an agent-triggered headless command without screenshots being the sole evidence.

**Done when:** a developer can toggle F1 in a normal game, select a body, inspect meaningful geometry and its eligibility reasons, and compare presented versus tick shapes without opening source. A headless agent can retrieve the same facts. Default debug remains off for normal gameplay.

### GOBS-5 — coverage regression and cost gates

1. Add focused fixture tests for installed providers, room scoping, normal and inactive/disabled geometry, fallback/intangible hurtbox semantics, contact and subject-based filters, interaction priority, reflection/portal-part transforms and rollback-aware stamps where applicable.
2. Verify that a new geometry-contributing capability with a declared observation contract automatically appears in catalog/layer selection. Missing adapter/unknown role produces a discoverable diagnostic rather than a silent absence.
3. Measure disabled-mode overhead (should be effectively zero diagnostic geometry work), enabled frame cost in a dense melee and multi-room scene, observation cardinality, truncation behavior and memory retention. Bound report size and label drops explicitly. Don't require broad integrity scans every frame.
4. Confirm optional rendering/debug adapters are not required by headless simulation, and release builds remain usable without F1. If a target cannot support some debug render feature, headless observation still works.

**Done when:** fixture tests and performance reports are reproducible from documented commands, and a missing supported producer cannot quietly ship without a discoverable visibility failure. The acceptance suite checks user-visible results, not just enum population or minimum counts.

## Dependency and non-goals

- **Depends on:** existing `CombatGeometryView`, `CollisionWorld`, `RetainedDrawings`, developer mode presets, room/view identities and the read-only inspection/discovery contract. It does not require a new ECS, new general physics engine, or the entire editor.
- **Coordinates with:** collision/CCD, interaction, navigation, portal similarity and moveset inspector owners. They retain authority for their mechanics; GOBS owns only observation coverage and presentation.
- **Not required:** a Godot-like scene hierarchy, runtime editing of collision truth from the F1 overlay, universal reflective discovery of arbitrary Rust types, automatic visualization of invisible game rules that do not involve geometry, persistent debug entities per shape, and global spatial-semantic unification.
- **No hiding in coarse rectangles:** AABB approximation is useful for broadphase diagnosis only when labeled. Actual narrowphase, portal-part and authored geometry remains queryable and visible at the level needed to explain outcomes.
- **No invented metrics:** collect measurements in the implementation branch before selecting thresholds; if a feature is omitted for cost, its omission is reported explicitly.

## Final acceptance scenarios

A reviewer should be able to run a single documented lab and answer these without reading implementation code:

1. Why did one character collide with this wall while another passed through its portal cutout?
2. Why didn't this sword hit an enemy despite the animated sprite appearing to reach it?
3. Why is this hurtbox absent (intangible) or different from the coarse body collider?
4. Why can't I talk to this nearby NPC or interact with this chest, door or switch?
5. Which navigation edges and reachable surfaces did the actor use to select its path?
6. Where do a multi-portal boss's transformed pieces, hit volumes and clipping boundaries actually exist?
7. Are the apparent collision offsets caused by incorrect simulation geometry or by presentation interpolation?
8. How do I inspect the same exact shapes and conditions from a headless tool?

All answers must name the current live-room/subject/time context and point to the producer of the decision. An unsupported producer is explicitly reported, never quietly absent. Keep this packet in the active queue until the coverage matrix, end-to-end lab and regression gates are complete; do not substitute a count of colored rectangles for semantic coverage.
