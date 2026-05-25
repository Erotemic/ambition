# Optional actor contract sidecars

The renderer now emits an additional optional sidecar next to generated sprite
sheets:

```text
<stem>_spritesheet.png
<stem>_spritesheet.yaml
<stem>_spritesheet.ron   # existing runtime sheet layout
<stem>_actor.ron         # new optional actor contract, ignored by current runtime
```

The intent is to start populating the renderer -> engine contract before the
sandbox consumes it. The existing `SheetRegistry` contract remains unchanged.

## Design posture

The contract is sparse. Every character may omit any capability, socket, body
hint, animation binding, or interaction hook. Missing fields are not errors by
themselves; enabled capabilities/actions create requirements, and validation
will eventually check whether those requirements are satisfied by explicit data
or by documented fallbacks.

This is important for non-humanoid characters: a zombie does not need a
`hand_r`, a flying shark does not need feet-based locomotion, a portrait may
have no traversal, and a multipart boss may not have one rectangular body.

## Current RON shape

The first sidecar schema intentionally uses string identifiers and maps for
open vocabularies:

```ron
(
    schema_version: 1,
    character_id: "npc_erdish",
    actor_id: None,
    display_name: Some("Erdish"),
    provenance: Some((
        surface: "adapter",
        renderer_target: "toon",
        output_stem: "erdish",
        seed: 731,
        archetype: "erdish",
        variant: None,
        held_item: None,
        source_config: ".../configs/review/erdish.yaml",
    )),
    visual: Some((
        sheet_id: "erdish",
        spritesheet: "erdish_spritesheet.png",
        sheet_manifest: "erdish_spritesheet.ron",
        default_pose: Some("idle"),
        facing_policy: None,
        scale: None,
    )),
    body: Some((
        body_kind: Some("Standard"),
        body_plan: Some("HumanoidBiped"),
        collision: Some((w_px: 28.0, h_px: 46.0, source: "sheet.body_metrics.body_pixel_bbox", confidence: "derived")),
        hurtbox: Some((w_px: 28.0, h_px: 46.0, source: "derived_from_collision", confidence: "fallback")),
        mass_class: Some("Medium"),
        locomotion_hint: Some("Walk"),
        body_metrics_source: Some("sheet.body_metrics"),
        traits: [],
    )),
    capabilities: Some((
        traversal: Some((walk: Some(true), jump: None, climb: None, fly: None, swim: None, crawl: None, use_lifts: None, door_access: [])),
        interactions: Some((talk: Some(true), trade: None, carry: None, open_doors: [])),
    )),
    brain: Some((default_preset: Some("patrol_peaceful"))),
    actions: Some((default_preset: Some("peaceful"))),
    animation_bindings: {
        "default": (animation: "idle", events: []),
        "locomotion.walk": (animation: "walk", events: []),
        "interaction.talk": (animation: "talk", events: []),
    },
    sockets: {
        "feet": (source: "body_metrics.feet_pixel", animation: None, frame: None, point: (x: 44.5, y: 111.0)),
        "head": (source: "body_metrics.body_pixel_bbox", animation: None, frame: None, point: (x: 45.0, y: 4.0)),
    },
    tags: ["intro", "story"],
    missing_information: [
        "collision: not authored; engine should derive from sheet body_metrics or LDtk AABB",
        "socket hand_r: absent unless renderer provides it; actions must use fallback or another socket",
    ],
)
```

## Population sources

Adapter/YAML targets can author any of these loose blocks:

```yaml
actor:
  character_id: npc_zombie_shambler
  actor_id: erdish
  display_name: Erdish
visual:
  default_pose: shamble_idle
body:
  body_plan: HumanoidBiped
  body_kind: Standard
  traits: [undead, no_hands]
capabilities:
  traversal:
    walk: true
    jump: null
  interactions:
    talk: true
brain:
  default_preset: melee_brute_slow
actions:
  default_preset: zombie_bite
animation_bindings:
  action.melee.primary:
    animation: bite
sockets:
  mouth:
    point: {x: 22.0, y: 26.0}
missing_information:
  - "bite active frames still renderer-defaulted"
```

Every registered character now carries its own authored metadata at the
character authoring surface:

- YAML/adapter characters put sparse `actor`, `body`, `capabilities`, `brain`,
  `actions`, `animation_bindings`, and `sockets` blocks directly in the YAML.
- Single Python tack-ons expose module-level `ACTOR_METADATA`.
- Multi-target Python modules attach per-target `actor_metadata` inside their
  local `TARGETS` table.

Omitted low-level geometry is still derived conservatively from generated sheet
rows and body metrics. The contract builder currently populates several
runtime-useful fallback fields automatically:

- collision and fallback hurtbox rectangles from `body_metrics.body_pixel_bbox`
  when a non-boss/non-prop body has opaque pixels,
- mass class from body plan/body kind/tags,
- `feet`, `root`, `center`, `head`, and `chest` sockets from body metrics,
- first-class sockets from any per-frame `rects[*].anchors`,
- heuristic humanoid hand/weapon sockets when the body plan actually supports
  hands,
- mouth sockets for bite/beast/crawler/flyer-like characters, and
- family/default traversal and interaction capabilities for known renderer
  targets such as robot, goblin, sandbag, boss, prop, flyer, and crawler.

`missing_information` is now conditional: it records facts that are still truly
absent for the enabled capabilities/actions, not every possible future field.

Tack-on targets should expose module-level `ACTOR_METADATA` for single-target
modules, or pass/attach per-target `actor_metadata` for multi-target modules.
The test suite fails if a registered character target has no local metadata.

## Known gaps to fill over time

- Authoritative collision/hurtbox dimensions when opaque-pixel derivation is
  not good enough for gameplay.
- Measured traversal values such as jump height/distance, climb affordances,
  crawl clearance, and door/lift permissions.
- Per-frame sockets from rigs (`hand_r`, `weapon_tip`, `muzzle`, `mouth`, etc.)
  instead of only approximate `feet/head/center` sockets from body metrics.
- Authoritative action animation events from each generator instead of default
  timeline guesses for melee/ranged rows.
- Replacing remaining approximate socket coordinates with measured rig/anchor
  output from the drawing code itself.

