# Combat Inspection and Moveset Observatory

**Owner:** deterministic combat inspection. All ten exit criteria below hold, so
the observability architecture is complete. The remaining work is incremental:
M3 art/geometry agreement, the untraced consequence kinds in M5, and gathering
renders into the M7 bundle. The tool's own design notes are in
[`tools/ambition_moveset_inspector/docs/inspector.md`](../../tools/ambition_moveset_inspector/docs/inspector.md).

The [architecture reassessment](engine/architecture-reassessment.md) adds two
constraints: discovery uses the actual installed/prepared technique catalog, and
contact inspection consumes A2's accepted target/geometry fact rather than
reconstructing an independent hit decision. Cached takes remain evidence, not
content authority. A11/A12 reject invalid keys/flows before a take is scheduled.

## Purpose

Give a human or LLM a canonical inspection surface for authored combat behavior,
so they can answer: what move was authored; what the runtime resolved; where the
actor and target were; where attack and damageable geometry existed; what
contacted what; what consequences the contact produced; what animation,
projectile, summon, audio and VFX accompanied the move; what follow-ups were
available; and how a change altered those results.

The loop this serves:

```text
discover
→ author
→ execute representative scenario
→ inspect runtime consequences
→ compare against intent/baseline
→ refine
→ validate in the actual game
```

`tools/ambition_moveset_inspector` is the first frontend. The capability is an
Engine 1.0 authoring and observability capability, not only a Smash tool; combat
is its first customer because geometry, timing, animation, state and effects
interact densely. Do not generalize it to NPC actions, traps or encounter
mechanics until a real customer needs that.

## Architectural principles

1. **The runtime is authoritative.** Do not implement an alternate combat
   evaluator in the inspector. Construct deterministic scenarios, run the real
   systems, and publish semantic read models. If the runtime cannot expose a fact
   cleanly, improve the runtime's observability rather than duplicating its logic.
2. **Coarse visualization is acceptable; inaccurate geometry is not.** A simple
   canvas with faithful runtime volumes beats a polished render with
   reconstructed geometry.
3. **Cached artifacts are evidence, not authority.** The roster and move
   inventory come from prepared content. A missing cache entry must not make a
   fighter disappear.
4. **Inspection scenarios are semantic objects.** The unit is a deterministic
   scenario (subject, target, move, posture, spacing, damage, charge, target
   behavior, environment, horizon) that produces one observation artifact.
5. **Subject and target are explicit.** Roles (`Subject`, `Target`,
   `SubjectOwned`, `TargetOwned`, `Other`) appear in data and presentation.
6. **Runtime geometry is the comparison standard.** Compare art to combat
   geometry; never infer combat geometry from art.
7. **Machine-readable inspection is a first-class product.** A CLI or library
   caller gets compact structured results without a UI.

Two activation modes must stay distinct, and every report states which it used:
**input-path inspection** (input → gesture → action resolution → acceptance →
behavior) and **direct resolved-move inspection** through a sanctioned
diagnostic activation path that bypasses input mapping but not the move runtime.
Do not mutate playback state ad hoc from a tool.

## The stack

| piece | path | role |
|---|---|---|
| moveset export | `game/ambition_app_tools/src/bin/moveset_export.rs` | prepared roster, moves, and the resolved cancel graph |
| move driver | `crates/ambition_sim_harness/src/move_exercise.rs` | the one authority for "perform this move"; both drivers use it alone |
| take recorder | `game/ambition_app_tools/src/bin/moveset_takes.rs` | cheap full-tick recorder over the real Smash host, `NoWindow` |
| geometry read model | `crates/ambition_sim_view/src/combat_geometry_view.rs` (`CombatGeometryView`) | the only source of attack and damageable geometry |
| GPU renderer | `game/ambition_app_tools/src/bin/moveset_render.rs`, `crates/ambition_render/src/capture.rs`, `crates/ambition_render/src/rendering/debug_viz.rs` | real art with the production combat overlay, from the same execution |
| report | `scripts/moveset_report.py` | measurements, consequence chain, bundle, `--against` diff |
| diagnostic sheet | `scripts/render_take_diagnostic.py` | key-frame filmstrip |
| inspector | `tools/ambition_moveset_inspector/` (server `ambition_moveset_inspector/server.py`, browser `web/app.js`) | draws recorded observations; derives no geometry |

Artifact identity (`SimId`, `MatchSeat`, `WornCharacter`) may be joined at the
tool boundary; geometry and combat state come from the sim-view.

## Standing rules from the closed milestones

- **Default target.** The default target is `sandbag_infinite`, a passive
  immortal dummy, so the same move on two fighters is measured against one body.
  The value lives in three places that must agree:
  `ambition_demo_smash::INSPECTION_TARGET`, `DEFAULT_SCENARIO_TARGET` in the
  inspector server, and the same constant in `web/app.js`. A mirror or any real
  fighter stays selectable as a distinct scenario.
- **Overlap is not contact.** `overlap_ticks` (boxes intersecting) and `contacts`
  (the runtime's hit-once memory) are separate lines; the summary warns when the
  first is nonzero and the second is zero.
- **A layer toggle changes what is drawn, never what is measured.**
  `moveset_render --overlay on|off|art,hurtboxes,strikes` drives the layers
  through one definition (`dev_tools::force_combat_overlay` with
  `CombatOverlayLayers`), and the manifest records which were on. The `Combat`
  debug preset turns on a combined gate, so selecting one layer means clearing it.
- **Key frames reuse the report's measurements** (`moveset_report.measure`) rather
  than deriving "first contact" a second way. An even strip usually misses a
  short active window.
- **Consequences are differenced from what the runtime published**, never
  recomputed from a knockback formula. With `--features causal`, the engine's own
  resolution (`BodyHitResolved`, `BodyReactionApplied`) is reported.
- **A chain probe's schedule is a pure function of the action tick.**
  `--chain-at` is an input to sweep; a probe that waited for A to connect would
  make the press depend on the outcome it measures. "The engine never played B"
  is an answer and is reported as one.
- **The exporter resolves the cancel graph, not the browser.**
  `MovesetContract::cancel_targets` matches on `cancel_names_for`, the same list
  the trigger road uses, and the namespace is total: every verb arm answers.
- **Provenance.** Reports name the source recording, its mtime and all schema
  versions; `--against` refuses to present two different scenarios as one change.
  A stale render stays visibly stale.

## Open work

### M3 — art/geometry agreement

The overlay half is closed: one PNG carries real art, target, VFX and
`CombatGeometryView` volumes from one execution. Remaining:

- **Define agreement in pixels:** which anchor, what tolerance, at which zoom.
  This is a definition, not a seam, and it comes first.
- **Measure the frame with the camera that drew it.** Use Bevy's
  `Camera::world_to_viewport`, as `capture_sanic.rs` already does to prove a
  subject is in frame. Do not compose a parallel world-to-screen helper from the
  sim-view camera components: those are the simulation's intent, and a prediction
  built from them can disagree with the picture and be scored as an art failure.
- **Missing overlay gates:** trajectories, contact markers and VFX have no
  independent layer.
- **Agreement measurements to support:** visual weapon tip versus attack extent;
  attack volume mostly inside the body; visible attack with no active volume;
  active volume before the visual action; VFX centre versus contact point;
  projectile sprite versus projectile collision volume. Expose evidence; do not
  "correct" authored geometry automatically.

### M5 — consequence tracing

Contacts, damage, hitstun, hitlag, launch and displacement are traced. Status
effects, VFX and SFX are not.

### M7 — artifact bundle

`moveset_report.py --out DIR` writes `report.json` (the machine-readable
authority), `summary.md`, `trace.jsonl` and `filmstrip.svg`. The GPU frames that
`moveset_render` writes are not yet gathered into the bundle's `render/`
directory. Visual diffing (before/after geometry paths, contact locations,
trajectories) is later work.

### Browser organization

Organize the workspace around Fighter → Move → Scenario rather than around
recorded artifact types. "Engine Take" is a cache concept, not an author-facing
model. Primary pane: a synchronized viewport with subject, target, art, runtime
geometry, timeline and key events. Supporting panes: move metadata,
measurements, cancel graph, consequence trace, provenance and before/after
comparison.

## Rendering and corpus policy

Bulk high-quality rendering does not need to be interactive. A full-grid redraw
of about half an hour is acceptable for overnight campaigns, regression
snapshots, release checkpoints and GPU-host generation. The fast loop is one move,
one character, one scenario, with coarse faithful geometry. Use a GPU host when it
helps; do not redesign around weak local rendering hardware, and do not optimize
the full-grid path before it is needed.

## Integration with other planning owners

- [`authoring-and-tools.md`](engine/authoring-and-tools.md) links here for combat
  authoring/inspection rather than duplicating it.
- [`inspection-diagnostics-and-workbench.md`](engine/inspection-diagnostics-and-workbench.md):
  use generic pause/step, scenario execution, query, trace and overlay
  facilities where they exist; do not add combat-only infrastructure when the
  generic capability is reusable.
- [`sprite-renderer.md`](engine/sprite-renderer.md): the melee hitbox-agreement
  question resolves toward displaying and measuring authoritative runtime
  geometry over rendered art. The sprite tool is not an authority for combat
  volumes.
- Smash parity/training: the live hitbox/hurtbox overlay uses this capability; do
  not build a Smash-specific geometry implementation.

## Exit criteria

| # | criterion | status |
|---|---|---|
| 1 | every fighter/move selectable independently of cached artifacts | holds |
| 2 | a deterministic scenario inspects subject and target through the real runtime | holds: `--target`, `--target-behavior passive`, `--spacing` |
| 3 | attack and damageable geometry published from runtime authority | holds: `CombatGeometryView` only, guarded by an absence contract |
| 4 | contacts and consequences machine-readable | holds; with `--features causal`, includes the engine's own resolution |
| 5 | rendered evidence aligned with the same semantic scenario | holds: one execution, overlay plus shutter-time observation |
| 6 | agents generate and consume a compact artifact noninteractively | holds: `moveset_takes --verbs`, `moveset_report.py`, SVG sheets |
| 7 | before/after behavioural comparison | holds: `--against` |
| 8 | the browser consumes the same semantic artifacts | holds: it draws the recorded observation |
| 9 | representative move-chain inspection | holds: resolved cancel graph and the empirical A→B probe |
| 10 | the major remaining work is UX/coverage/performance, not observability | holds |

The end state is not a prettier moveset browser. It is a combat observatory that
makes authored mechanics empirically inspectable.

## Tests and policy guards

Extend these rather than replacing them:

```text
tools/ambition_moveset_inspector/check_browser_acceptance.mjs
tools/ambition_moveset_inspector/check_bundle_contract.mjs
tools/ambition_moveset_inspector/check_draw_path.mjs
scripts/tests/test_moveset_inspector_renderer.py
scripts/check_absence_contracts.py
```

| # | requirement | where it lives |
|---|---|---|
| 1 | N fighters produce N selectable fighters with zero takes | `check_takes_discovery.mjs` |
| 2 | two recordings do not limit the picker to two | `check_takes_discovery.mjs` |
| 3 | subject/target ids and roles survive serialization | `combat_observation::tests::a_seated_scenario_serializes_roles_identities_and_both_geometries` |
| 4 | a published empty `DamageableVolumes` produces no hurtbox | `the_artifact_distinguishes_intangible_from_a_coarse_fallback` (`ambition_sim_harness/tests/combat_observation_it.rs`) |
| 5 | unpublished damageable geometry falls back to the coarse box | same test |
| 6 | circle/OBB/convex strike geometry survives serialization | `every_volume_shape_survives_serialization`, `test_a_strike_is_drawn_in_its_real_shape` |
| 7 | target-owned strikes are not attributed to the subject | `a_strike_belongs_to_its_owners_side_not_to_the_owner` |
| 8 | a rendered frame and its semantic manifest name the same action tick | structural: the observation is a field of the shot row, beside `action_tick` |
| 9 | a stale render stays visibly stale | `test_a_cache_older_than_the_binary_is_not_served`, `test_a_cache_with_no_renderer_provenance_is_not_served_as_current` |
| 10 | both drivers use `move_exercise` alone | absence contract `the-two-move-drivers-do-not-author-their-own-presses` |

## Non-goals

Do not create a new combat simulator, a second hitbox or hurtbox resolver, or a
third move-input driver. Do not replace `moveset_export`, `moveset_takes`,
`moveset_render` or `DeterministicCaptureSession`, and do not build a new web
server. Do not optimize the full-grid run or add a persistent renderer daemon
before the one-move workflow is measured. Do not add combo classification,
general scene editing or visual scripting to this program.
